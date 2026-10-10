use crate::game::level::rectiter::{MutableRectIterator, RectIterator};
use crate::game::level::terrain::{TER_BIT_WATER, is_space};
use crate::game::level::{LevelInfo, PreloadedLevel, parse_palette_mapping, terrain};
use crate::gfx::{Color, Image, ImageAlign};
use crate::math::Vec2;
use crate::{game::level::TILE_SIZE, math::Rect};
use anyhow::{Result, anyhow};
use core::ops::Deref;
use itertools::izip;
use mlua::{Lua, LuaSerdeExt, Table, UserData};
use sdl3_sys::pixels::{SDL_PIXELFORMAT_ARGB8888, SDL_PIXELFORMAT_INDEX8, SDL_PixelFormat};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

struct ProceduralLevel {
    artwork: Image,
    terrain: Image,
    config: mlua::Value,
    settings: mlua::Value,
    water_color: Color,
    rng: fastrand::Rng,
}

#[derive(Clone)]
struct ImagePart {
    artwork: Image,
    terrain: Image,
    rect: Rect,
}

#[derive(Clone)]
struct HoleTemplate(Image);

#[derive(Copy, Clone)]
pub enum BlitMode {
    Over,
    Under,
}

pub fn generate_procedural_level(info: LevelInfo) -> Result<PreloadedLevel> {
    // Create Lua environment for generating the level
    let lua = Lua::new();

    {
        let globals = lua.globals();
        // Load modules from the script path only
        globals.get::<Table>("package")?.set(
            "path",
            format!("{}/?.lua", info.root_path().to_str().unwrap()),
        )?;

        // Add the level we're working on
        globals.set("level", ProceduralLevel::new(&lua, &info)?)?;

        // Image loading tools
        let images = lua.create_table()?;
        {
            let root_path = info.root_path().to_owned();
            images.set(
                "load_set",
                lua.create_function(move |lua, filename: String| {
                    Ok(load_imageset(lua, root_path.join(filename))?)
                })?,
            )?;
        }
        {
            let root_path = info.root_path().to_owned();
            images.set(
                "load_hole_template",
                lua.create_function(move |_lua, filename: String| {
                    Ok(HoleTemplate::load(root_path.join(filename))?)
                })?,
            )?;
        }

        images.set(
            "new_hole_template",
            lua.create_function(move |_lua, (w, h): (i32, i32)| Ok(HoleTemplate::new(w, h)?))?,
        )?;

        globals.set("images", images)?;

        // Common types
        globals.set(
            "Vec2",
            lua.create_function(|_, (x, y): (f32, f32)| Ok(Vec2(x, y)))?,
        )?;

        globals.set(
            "Rect",
            lua.create_function(|_, (x, y, w, h): (i32, i32, i32, i32)| Ok(Rect::new(x, y, w, h)))?,
        )?;
    }

    // Run level creation script
    let script_name = info
        .generation_script_name()
        .expect("script name shoulstruct ProceduralLevel {}d have been set");
    lua.load(format!(
        r#"require "{}""#,
        &script_name[0..script_name.len() - 4]
    ))
    .exec()?;

    // Done. Extract the finished level
    let level = lua.globals().get::<ProceduralLevel>("level")?;

    Ok(PreloadedLevel {
        artwork: level.artwork.clone(),
        terrain: level.terrain.clone(),
        script_settings: lua.from_value(level.settings)?,
        info,
    })
}

impl ProceduralLevel {
    fn new(lua: &Lua, info: &LevelInfo) -> Result<Self> {
        // Width and height are set in script config (but we have fallbacks in case they aren't)
        let width = info
            .script_config()
            .get("width")
            .unwrap_or(&toml::Value::Integer(1024))
            .as_integer()
            .ok_or_else(|| anyhow!("width is not an integer!"))? as i32;

        let height = info
            .script_config()
            .get("height")
            .unwrap_or(&toml::Value::Integer(1024))
            .as_integer()
            .ok_or_else(|| anyhow!("height is not an integer!"))? as i32;

        // round up sizes to a multiple of TILE_SIZE
        let width = (width + TILE_SIZE - 1) / TILE_SIZE * TILE_SIZE;
        let height = (height + TILE_SIZE - 1) / TILE_SIZE * TILE_SIZE;

        let rng_seed: Option<i64> = info
            .script_config()
            .get("seed")
            .and_then(|v| v.as_integer());

        Ok(ProceduralLevel {
            artwork: Image::blank(width, height, SDL_PIXELFORMAT_ARGB8888)?,
            terrain: Image::blank(width, height, SDL_PIXELFORMAT_INDEX8)?,
            config: lua.to_value(info.script_config())?,
            settings: lua.to_value(info.script_settings())?,
            water_color: Color::from_argb_u32(info.colors().water.unwrap_or(0xff0000ff)),
            rng: if let Some(seed) = rng_seed {
                fastrand::Rng::with_seed(seed as u64)
            } else {
                fastrand::Rng::new()
            },
        })
    }

    fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.artwork.width(), self.artwork.height())
    }

    fn random_points_in(&mut self, rect: Rect, count: usize) -> Vec<Vec2> {
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            list.push(Vec2(
                rect.x() as f32 + self.rng.f32() * rect.w() as f32,
                rect.y() as f32 + self.rng.f32() * rect.h() as f32,
            ));
        }
        list
    }

    fn put_image(&mut self, x: i32, y: i32, image: &ImagePart, align: ImageAlign, mode: BlitMode) {
        let offset = align.offset(image.rect.w(), image.rect.h());
        let target = (offset.0 + x, offset.1 + y);

        let op: CompositeModeFn = match mode {
            BlitMode::Over => |asrc, tsrc, adest, tdest| {
                if !is_space(tsrc) {
                    *tdest = tsrc;
                }
                *adest = Color::blend_argb8888(*adest, asrc);
            },
            BlitMode::Under => |asrc, tsrc, adest, tdest| {
                if !is_space(tsrc) && is_space(*tdest) {
                    *tdest = tsrc;
                }
                *adest = Color::blend_argb8888(asrc, *adest);
            },
        };

        blit_level_part(
            &image.artwork,
            &image.terrain,
            &mut self.artwork,
            &mut self.terrain,
            &image.rect,
            target,
            op,
        );
    }

    fn make_hole(
        &mut self,
        x: i32,
        y: i32,
        template: &HoleTemplate,
        align: ImageAlign,
        threshold: u8,
    ) {
        let width = template.0.width();
        let height = template.0.height();

        let offset = align.offset(width, height);
        let target = (offset.0 + x, offset.1 + y);

        // Destination must be at least partially inside the dest image
        let dest_rect =
            match Rect::new(target.0, target.1, width, height).intersected(self.artwork.bounds()) {
                Some(d) => d,
                None => {
                    // Not an error but may indicate a bug in the generation script
                    log::warn!(
                        "Tried to make a hole fully outside level bounds ({}, {})",
                        target.0,
                        target.1
                    );
                    return;
                }
            };

        // Target may be partially outside level bounds
        let source_rect = Rect::new(
            -target.0.min(0),
            -target.1.min(0),
            dest_rect.w(),
            dest_rect.h(),
        );

        let tpl_iterator = RectIterator::from_rect(
            template
                .0
                .indexed_pixels()
                .expect("hole template pixel format should be index8"),
            template.0.width() as usize,
            &source_rect,
        );

        let terrain_stride = self.terrain.width() as usize;
        let terrain_iterator = MutableRectIterator::from_rect(
            self.terrain
                .indexed_pixels_mut()
                .expect("terrain pixel format should be index8"),
            terrain_stride,
            &dest_rect,
        );

        let artwork_stride = self.artwork.width() as usize;
        let artwork_iterator = MutableRectIterator::from_rect(
            self.artwork
                .argb8888_pixels_mut()
                .expect("artwork pixel format should be argb8888"),
            artwork_stride,
            &dest_rect,
        );

        for ((tpl_row, (terrain_row, _)), (artwork_row, _)) in
            tpl_iterator.zip(terrain_iterator).zip(artwork_iterator)
        {
            for (tpl, (ter, art)) in tpl_row
                .iter()
                .zip(terrain_row.iter_mut().zip(artwork_row.iter_mut()))
            {
                if *tpl > threshold {
                    //*art = *art & 0x00ffffff | ((255 - *tpl as u32) << 24);
                    *art = 0;
                    *ter = 0;
                }
            }
        }
    }

    fn put_water(&mut self, area: Rect, water_color: Color, alpha: f32) {
        let area = match area.intersected(self.bounds()) {
            Some(r) => r,
            None => {
                log::warn!("put_water({}) fully out of bounds!", area);
                return;
            }
        };

        let terrain_stride = self.terrain.width() as usize;
        let terrain_iterator = MutableRectIterator::from_rect(
            self.terrain
                .indexed_pixels_mut()
                .expect("terrain pixel format should be index8"),
            terrain_stride,
            &area,
        );

        let artwork_stride = self.artwork.width() as usize;
        let artwork_iterator = MutableRectIterator::from_rect(
            self.artwork
                .argb8888_pixels_mut()
                .expect("artwork pixel format should be argb8888"),
            artwork_stride,
            &area,
        );

        let water_overlay = water_color.with_alpha(alpha).as_premultiplied_argb_u32();
        let water_color = water_color.as_premultiplied_argb_u32();

        for ((ter_row, _), (art_row, _)) in terrain_iterator.zip(artwork_iterator) {
            for (t, a) in ter_row.iter_mut().zip(art_row.iter_mut()) {
                *t |= TER_BIT_WATER;
                if *a == 0 {
                    *a = water_color;
                } else {
                    *a = Color::blend_argb8888(*a, water_overlay);
                }
            }
        }
    }

    fn terrain_at(&self, pos: Vec2) -> terrain::Terrain {
        let w = self.terrain.width();
        let x = pos.0 as i32;
        let y = pos.1 as i32;
        if x < 0 || y < 0 || x >= w || y >= self.terrain.height() {
            return terrain::TER_LEVELBOUND;
        }

        let pixels = self.terrain.indexed_pixels().expect("should be index8");
        pixels[(y * w + x) as usize]
    }
}

type CompositeModeFn = fn(u32, u8, &mut u32, &mut u8);
fn blit_level_part(
    artwork_src: &Image,
    terrain_src: &Image,
    artwork_dest: &mut Image,
    terrain_dest: &mut Image,
    source_rect: &Rect,
    target: (i32, i32),
    op: CompositeModeFn,
) {
    // Destination must be at least partially inside the dest image
    let dest_rect = match Rect::new(target.0, target.1, source_rect.w(), source_rect.h())
        .intersected(artwork_dest.bounds())
    {
        Some(d) => d,
        None => {
            // Not an error but may indicate a bug in the generation script
            log::warn!(
                "Tried to blit fully outside level bounds ({}, {})",
                target.0,
                target.1
            );
            return;
        }
    };

    // Target may be partially outside level bounds
    let source_rect = Rect::new(
        source_rect.x() - target.0.min(0),
        source_rect.y() - target.1.min(0),
        dest_rect.w(),
        dest_rect.h(),
    );

    let terrain_src_iterator = RectIterator::from_rect(
        terrain_src
            .indexed_pixels()
            .expect("terrain pixel format should be index8"),
        terrain_src.width() as usize,
        &source_rect,
    );

    let artwork_src_iterator = RectIterator::from_rect(
        artwork_src
            .argb8888_pixels()
            .expect("artwork pixel format should be argb8888"),
        artwork_src.width() as usize,
        &source_rect,
    );

    let terrain_dest_stride = terrain_dest.width() as usize;
    let terrain_dest_iterator = MutableRectIterator::from_rect(
        terrain_dest
            .indexed_pixels_mut()
            .expect("terrain pixel format should be index8"),
        terrain_dest_stride,
        &dest_rect,
    );

    let artwork_dest_stride = artwork_dest.width() as usize;
    let artwork_dest_iterator = MutableRectIterator::from_rect(
        artwork_dest
            .argb8888_pixels_mut()
            .expect("artwork pixel format should be argb8888"),
        artwork_dest_stride,
        &dest_rect,
    );

    for (terrain_src_row, artwork_src_row, (terrain_dest_row, _), (artwork_dest_row, _)) in izip!(
        terrain_src_iterator,
        artwork_src_iterator,
        terrain_dest_iterator,
        artwork_dest_iterator
    ) {
        for (tsrc, asrc, tdest, adest) in izip!(
            terrain_src_row,
            artwork_src_row,
            terrain_dest_row,
            artwork_dest_row
        ) {
            op(*asrc, *tsrc, adest, tdest);
            /*
            if !is_space(*tsrc) {
                *tdest = *tsrc;
                *adest = *asrc;
            }
            */
        }
    }
}

impl UserData for ProceduralLevel {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.terrain.width()));
        fields.add_field_method_get("height", |_, this| Ok(this.terrain.height()));
        fields.add_field_method_get("config", |_, this| Ok(this.config.clone()));
        fields.add_field_method_get("settings", |_, this| Ok(this.settings.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "put_image",
            |_, this, (x, y, image, align, mode): (i32, i32, ImagePart, ImageAlign, BlitMode)| {
                this.put_image(x, y, &image, align, mode);
                Ok(())
            },
        );

        methods.add_method_mut(
            "make_hole",
            |_,
             this,
             (x, y, template, align, threshold): (
                i32,
                i32,
                HoleTemplate,
                ImageAlign,
                Option<u8>,
            )| {
                this.make_hole(x, y, &template, align, threshold.unwrap_or(0));
                Ok(())
            },
        );

        methods.add_method_mut("put_water", |_, this, (rect, alpha): (Rect, f32)| {
            this.put_water(rect, this.water_color, alpha);
            Ok(())
        });

        // Sampling
        methods.add_method("terrain_at", |_, this, pos: Vec2| Ok(this.terrain_at(pos)));

        // Random number generation
        // We should use this instead of the Lua interpreter's RNG so that explicitly
        // choosing a seed will recreate the same levels.
        methods.add_method_mut("randi", |_, this, (min, max): (i32, i32)| {
            Ok(this.rng.i32(min..=max))
        });

        methods.add_method_mut("rand_points", |_, this, n: usize| {
            Ok(this.random_points_in(
                Rect::new(0, 0, this.terrain.width(), this.terrain.height()),
                n,
            ))
        });
        methods.add_method_mut("rand_points_in", |_, this, (rect, n): (Rect, usize)| {
            Ok(this.random_points_in(rect, n))
        });
    }
}

impl HoleTemplate {
    fn load(path: PathBuf) -> Result<Self> {
        Ok(Self(Image::from_file(path)?.ensure_index8()?))
    }

    fn new(width: i32, height: i32) -> Result<Self> {
        Ok(Self(Image::blank(width, height, SDL_PixelFormat::INDEX8)?))
    }

    fn draw_circle(&mut self, x: i32, y: i32, r: i32) {
        let hole_rect = match Rect::new(x - r, y - r, r * 2, r * 2).intersected(self.0.bounds()) {
            Some(r) => r,
            None => {
                log::warn!(
                    "Tried to draw circle outside template boundaries ({}, {})",
                    x,
                    y
                );
                return;
            }
        };

        let xoffset = hole_rect.x() - x;
        let yoffset = hole_rect.y() - y;
        let rr = r * r;

        let stride = self.0.width() as usize;
        for (j, (row, _)) in MutableRectIterator::new(
            self.0
                .indexed_pixels_mut()
                .expect("hole template should be index8"),
            stride,
            hole_rect.x() as usize,
            hole_rect.y() as usize,
            hole_rect.w() as usize,
            hole_rect.h() as usize,
        )
        .enumerate()
        {
            let j = j as i32 + yoffset;
            let jj = j * j;
            for (i, px) in row.iter_mut().enumerate() {
                let i = i as i32 + xoffset;
                if (jj + i * i) < rr {
                    *px = 0xff;
                }
            }
        }
    }

    fn draw_line(&mut self, src: Vec2, dest: Vec2, r: i32, step: f32) {
        let mut pos = src;
        let d = (dest - src).normalized() * step;
        let steps = (src.dist(dest) / step + 0.5) as i32;
        for _ in 0..steps {
            self.draw_circle(pos.0 as i32, pos.1 as i32, r);
            pos = pos + d;
        }
    }

    /// Box blur (can be repeated to approximate gaussian blur)
    fn blur(&mut self, blur_x: usize, blur_y: usize) {
        let w = self.0.width() as usize;
        let h = self.0.height() as usize;
        let pixels = self
            .0
            .indexed_pixels_mut()
            .expect("hole template should be index8");

        // Horizontal blur
        if blur_x > 0 {
            let radius = blur_x / 2;
            let mut buffer = vec![0; w];
            for row in pixels.chunks_mut(w) {
                let mut acc = row[1..radius]
                    .iter()
                    .fold(row[0] as usize * radius, |acc, i| acc + (*i as usize));
                for (i, b) in buffer.iter_mut().enumerate() {
                    *b = (acc / blur_x) as u8;
                    acc += row[(i + radius).min(w - 1)] as usize;
                    acc = acc.saturating_sub(row[i.saturating_sub(radius)] as usize);
                }
                row.copy_from_slice(&buffer[..]);
            }
        }

        // Vertical blur
        if blur_y > 0 {
            let radius = blur_y / 2;
            let mut buffer = vec![0; h];
            for col in 0..w {
                let mut acc = pixels[col..]
                    .iter()
                    .step_by(w)
                    .skip(1)
                    .take(radius - 1)
                    .fold(radius * pixels[col] as usize, |acc, i| acc + (*i as usize));
                for (j, b) in buffer.iter_mut().enumerate() {
                    *b = (acc / blur_y) as u8;
                    acc += pixels[col + (j + radius).min(h - 1) * w] as usize;
                    acc = acc.saturating_sub(pixels[col + j.saturating_sub(radius) * w] as usize);
                }
                pixels[col..]
                    .iter_mut()
                    .step_by(w)
                    .zip(&buffer)
                    .for_each(|(p, b)| *p = *b);
            }
        }
    }
}

impl mlua::FromLua for ProceduralLevel {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.take::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ProceduralLevel".to_owned(),
                message: Some("expected ProceduralLevel".to_string()),
            }),
        }
    }
}

impl UserData for HoleTemplate {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.0.width()));
        fields.add_field_method_get("height", |_, this| Ok(this.0.height()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("draw_circle", |_, this, (pos, r): (Vec2, i32)| {
            this.draw_circle(pos.0 as i32, pos.1 as i32, r);
            Ok(())
        });
        methods.add_method_mut(
            "draw_line",
            |_, this, (start, end, r): (Vec2, Vec2, i32)| {
                this.draw_line(start, end, r, r as f32 * 0.5);
                Ok(())
            },
        );

        methods.add_method_mut(
            "blur",
            |_, this, (rx, ry, repeat): (usize, Option<usize>, Option<i32>)| {
                for _ in 0..repeat.unwrap_or(1) {
                    this.blur(rx, ry.unwrap_or(rx));
                }
                Ok(())
            },
        );
    }
}

impl mlua::FromLua for HoleTemplate {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HoleTemplate".to_owned(),
                message: Some("expected HoleTemplate".to_string()),
            }),
        }
    }
}

impl UserData for ImagePart {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.rect.w()));
        fields.add_field_method_get("height", |_, this| Ok(this.rect.h()));
    }
}

impl mlua::FromLua for ImagePart {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ImagePart".to_owned(),
                message: Some("expected ImagePart".to_string()),
            }),
        }
    }
}

#[derive(serde::Deserialize)]
struct ImageSetToml {
    artwork: String,
    terrain: String,
    #[serde(rename = "terrain-palette")]
    terrain_palette: toml::Table,
    subimages: HashMap<String, (i32, i32, i32, i32)>,
}

fn load_imageset(lua: &Lua, mut path: PathBuf) -> Result<mlua::Table> {
    let content = read_to_string(&path)?;
    let info: ImageSetToml = toml::from_str(&content)?;

    path.pop();

    let artwork = Image::from_file(path.join(info.artwork))?
        .ensure_argb888()?
        .premultiplied()?;
    let mut terrain = Image::from_file(path.join(info.terrain))?.ensure_index8()?;

    // Do terrain palette mapping here, since different image sets
    // may have different mappings. Procedural level generation code works
    // in the game's internal palette format
    let palette = parse_palette_mapping(&info.terrain_palette)?;
    for p in terrain
        .indexed_pixels_mut()
        .expect("we called ensure_index8")
    {
        *p = palette[*p as usize];
    }

    let table = lua.create_table()?;
    for (name, rect) in info.subimages {
        table.set(
            name,
            ImagePart {
                artwork: artwork.clone(),
                terrain: terrain.clone(),
                rect: Rect::new(rect.0, rect.1, rect.2, rect.3),
            },
        )?;
    }

    Ok(table)
}

impl mlua::FromLua for BlitMode {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Nil => Ok(Self::Over),
            mlua::Value::String(s) => match s.as_bytes().deref() {
                b"over" => Ok(Self::Over),
                b"under" => Ok(Self::Under),
                unknown => {
                    Err(anyhow!("Unknown mode: {}", str::from_utf8(unknown).unwrap()).into())
                }
            },
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BlitMode".to_owned(),
                message: Some("expected BlitMode".to_string()),
            }),
        }
    }
}
