use crate::game::level::parse_palette_mapping;
use crate::game::level::rectiter::{MutableRectIterator, RectIterator};
use crate::game::level::terrain::is_space;
use crate::gfx::{Image, ImageAlign};
use crate::math::Vec2;
use crate::{game::level::TILE_SIZE, math::Rect};
use anyhow::{Result, anyhow};
use mlua::{Lua, LuaSerdeExt, Table, UserData};
use sdl3_sys::pixels::{SDL_PIXELFORMAT_ARGB8888, SDL_PIXELFORMAT_INDEX8};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

pub struct ProceduralLevel {
    pub artwork: Image,
    pub terrain: Image,
    pub script_settings: toml::Table,
}

struct ProceduralLevelLua {
    level: ProceduralLevel,
    config: mlua::Value,
    settings: mlua::Value,
    rng: fastrand::Rng,
}

#[derive(Clone)]
struct ImagePart {
    artwork: Image,
    terrain: Image,
    rect: Rect,
}

impl ProceduralLevel {
    fn new(width: i32, height: i32) -> Result<Self> {
        // round up sizes to a multiple of TILE_SIZE
        let width = (width + TILE_SIZE - 1) / TILE_SIZE * TILE_SIZE;
        let height = (height + TILE_SIZE - 1) / TILE_SIZE * TILE_SIZE;

        Ok(ProceduralLevel {
            artwork: Image::blank(width, height, SDL_PIXELFORMAT_ARGB8888)?,
            terrain: Image::blank(width, height, SDL_PIXELFORMAT_INDEX8)?,
            script_settings: toml::Table::new(),
        })
    }

    pub fn generate(
        root_path: PathBuf,
        script_name: &str,
        config: &toml::Table,
        settings: &toml::Table,
    ) -> Result<Self> {
        // Width and height are set in script config (but we have fallbacks in case they aren't)
        let width = config
            .get("width")
            .unwrap_or(&toml::Value::Integer(1024))
            .as_integer()
            .ok_or_else(|| anyhow!("width is not an integer!"))? as i32;

        let height = config
            .get("height")
            .unwrap_or(&toml::Value::Integer(1024))
            .as_integer()
            .ok_or_else(|| anyhow!("height is not an integer!"))? as i32;

        // Create Lua environment for generating the level
        let lua = Lua::new();

        {
            let globals = lua.globals();
            // Load modules from the script path only
            globals
                .get::<Table>("package")?
                .set("path", format!("{}/?.lua", root_path.to_str().unwrap()))?;

            // Add the level we're working on
            globals.set(
                "level",
                ProceduralLevelLua {
                    level: ProceduralLevel::new(width, height)?,
                    config: lua.to_value(config)?,
                    settings: lua.to_value(settings)?,
                    rng: fastrand::Rng::new(),
                },
            )?;

            // Image loading tools
            let images = lua.create_table()?;
            images.set(
                "load_set",
                lua.create_function(move |lua, filename: String| {
                    Ok(load_imageset(lua, root_path.join(filename))?)
                })?,
            )?;

            globals.set("images", images)?;

            // Common types
            globals.set(
                "Vec2",
                lua.create_function(|_, (x, y): (f32, f32)| Ok(Vec2(x, y)))?,
            )?;

            globals.set(
                "Rect",
                lua.create_function(|_, (x, y, w, h): (i32, i32, i32, i32)| {
                    Ok(Rect::new(x, y, w, h))
                })?,
            )?;
        }

        // Run level creation script
        lua.load(format!(
            r#"require "{}""#,
            &script_name[0..script_name.len() - 4]
        ))
        .exec()?;

        // Done. Extract the finished level
        let mut level = lua.globals().get::<ProceduralLevelLua>("level")?;
        level.level.script_settings = lua.from_value(level.settings)?;

        Ok(level.level)
    }

    fn put_image(&mut self, x: i32, y: i32, image: &ImagePart, align: ImageAlign) {
        let offset = align.offset(image.rect.w(), image.rect.h());
        let target = (offset.0 + x, offset.1 + y);
        image.artwork.blit(image.rect, &mut self.artwork, target);
        terrain_blit(&image.terrain, &mut self.terrain, &image.rect, target);
    }
}

/**
 * Copy terrain pixels while preserving space
 */
fn terrain_blit(source: &Image, dest: &mut Image, source_rect: &Rect, target: (i32, i32)) {
    // Destination must be at least partially inside the dest image
    let dest_rect = match Rect::new(target.0, target.1, source_rect.w(), source_rect.h())
        .intersected(Rect::new(0, 0, dest.width(), dest.height()))
    {
        Some(d) => d,
        None => {
            // Not an error but may indicate a bug in the generation script
            log::warn!(
                "Tried to blit terrain fully outside level bounds ({}, {})",
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

    let srci = RectIterator::from_rect(
        source
            .indexed_pixels()
            .expect("(source) terrain pixel format should be index8"),
        source.width() as usize,
        &source_rect,
    );

    let dest_stride = dest.width() as usize;
    let desti = MutableRectIterator::from_rect(
        dest.indexed_pixels_mut()
            .expect("(destination) terrain pixel format should be index8"),
        dest_stride,
        &dest_rect,
    );

    for ((d_row, _), s_row) in desti.zip(srci) {
        for (d, s) in d_row.iter_mut().zip(s_row.iter()) {
            if !is_space(*s) {
                *d = *s;
            }
        }
    }
}

impl ProceduralLevelLua {
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
}

impl UserData for ProceduralLevelLua {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.level.terrain.width()));
        fields.add_field_method_get("height", |_, this| Ok(this.level.terrain.height()));
        fields.add_field_method_get("config", |_, this| Ok(this.config.clone()));
        fields.add_field_method_get("settings", |_, this| Ok(this.settings.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "put_image",
            |_, this, (x, y, image, align): (i32, i32, ImagePart, ImageAlign)| {
                Ok(this.level.put_image(x, y, &image, align))
            },
        );

        // Random number generation
        // We should use this instead of the Lua interpreter's RNG so that explicitly
        // choosing a seed will recreate the same levels.
        methods.add_method_mut("rand_set_seed", |_, this, seed: u64| {
            this.rng.seed(seed);
            Ok(())
        });

        methods.add_method_mut("randi", |_, this, (min, max): (i32, i32)| {
            Ok(this.rng.i32(min..=max))
        });

        methods.add_method_mut("rand_points", |_, this, n: usize| {
            Ok(this.random_points_in(
                Rect::new(
                    0,
                    0,
                    this.level.terrain.width(),
                    this.level.terrain.height(),
                ),
                n,
            ))
        });
        methods.add_method_mut("rand_points_in", |_, this, (rect, n): (Rect, usize)| {
            Ok(this.random_points_in(rect, n))
        });
    }
}

impl mlua::FromLua for ProceduralLevelLua {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.take::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "ProceduralLevelLua".to_owned(),
                message: Some("expected ProceduralLevelLua".to_string()),
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

    let artwork = Image::from_file(path.join(info.artwork))?.ensure_argb888()?;
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
