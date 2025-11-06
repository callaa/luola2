// This file is part of Luola2
// Copyright (C) 2025 Calle Laakkonen
//
// Luola2 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Luola2 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Luola2.  If not, see <https://www.gnu.org/licenses/>.

use super::{LevelInfo, terrain};
use crate::{
    game::level::{
        terrain::TER_BIT_WATER,
        tileiterator::{MutableTileIterator, TileIterator},
    },
    gfx::{Color, Image, Renderer, Texture, TextureScaleMode},
    math::{Line, LineF, Rect, RectF, Vec2},
};

use anyhow::{Result, anyhow};
use either::Either;
use fastrand;
use log::error;
use mlua;
use sdl3_sys::pixels::SDL_PIXELFORMAT_ARGB8888;

pub const LEVEL_SCALE: f32 = 3.0; // Scaling factor: 1 level pixel equals this many world coordinates
pub const TILE_SIZE: i32 = 64;
pub const TILE_LENGTH: usize = (TILE_SIZE * TILE_SIZE) as usize;

/**
 * Knowing something about the content of the tile
 * allows us to skip certain checks in certain cases.
 *
 *  - Any subtractive level effect need only apply to Destructible tiles.
 *  - Collision checks can be skipped entirely for FreeSpace and Water tiles.
 *  - FreeSpace, Water and Indestructible tiles may turn into Destructible tiles if
 *    an additive level effect is applied.
 */
pub(super) enum TileContentHint {
    Destructible,   // may contain destructible terrain
    FreeSpace,      // contains only empty space
    Water,          // contains only water
    Indestructible, // may contain a mixture of space, water and/or indestructable solids
}

pub(super) struct TerrainTile {
    pub terrain: [terrain::Terrain; TILE_LENGTH], // used for collision checks
    pub artwork: [u32; TILE_LENGTH],              // used to update the artwork texture
    pub content_hint: TileContentHint,            // optimization hint
}

#[derive(Clone, Debug)]
pub struct Forcefield {
    /// The area (in world coordinates) of the field
    pub bounds: RectF,

    /// A uniform force applied to physical objects inside the field
    pub uniform_force: Vec2,

    /// Attractive or repulsive point force in the center of the field
    /// that falls off with distance in a physically inaccurate way.
    pub point_force: f32,

    /// ID value for updating the field after it's been created
    pub id: i32,
}

/**
 * The destructible terrain part of the game world.
 *
 * The level size is given in world coordinates, which is the internal size of the
 * level map multiplied by a scaling factor.
 * Internally, the level size must be a multiple of TILE_SIZE.
 */
pub struct Level {
    tiles: Vec<TerrainTile>, // length should be tiles_wide * tiles_high
    artwork: Texture,        // updated from tiles when changed
    background: Option<Texture>,
    width: f32,      // width in world coordinates
    height: f32,     // height in world coordinates
    tiles_wide: i32, // width in tiles
    tiles_high: i32, // height in tiles

    pub forcefields: Vec<Forcefield>,
    pub water_color: u32, // pixel value used when creating water
    pub snow_color: u32,  // pixel value used when creating snow
}

impl mlua::FromLua for Forcefield {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            Ok(Self {
                bounds: table.get("bounds")?,
                uniform_force: table.get::<Option<Vec2>>("uniform")?.unwrap_or_default(),
                point_force: table.get::<Option<f32>>("point")?.unwrap_or_default(),
                id: table.get("id")?,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Projectile".to_owned(),
                message: Some("expected a table describing a projectile".to_string()),
            })
        }
    }
}

impl Level {
    pub fn load_level(renderer: &Renderer, info: &LevelInfo) -> Result<Level> {
        let terrain = Image::from_file(info.terrain_path())?;

        if terrain.width() % TILE_SIZE > 0 || terrain.height() % TILE_SIZE > 0 {
            return Err(anyhow!(
                "Level dimensions not a multiple of 64 ({}x{})",
                terrain.width(),
                terrain.height()
            ));
        }

        let width = terrain.width() as f32 * LEVEL_SCALE;
        let height = terrain.height() as f32 * LEVEL_SCALE;
        let tiles_wide = terrain.width() / TILE_SIZE;
        let tiles_high = terrain.height() / TILE_SIZE;

        let artwork = Image::from_file(info.artwork_path())?.ensure_argb888()?;
        let transparent_color_index = if info.terrain_is_same_as_artwork() {
            info.transparent_color_index()
        } else {
            None
        };

        if artwork.width() != terrain.width() || artwork.height() != terrain.height() {
            return Err(anyhow!(
                "Level artwork size does not match terrain map size"
            ));
        }

        let background = if let Some(path) = info.background_path() {
            Some(Texture::from_file(renderer, path)?)
        } else {
            None
        };

        let mut tiles: Vec<TerrainTile> = Vec::with_capacity((tiles_wide * tiles_high) as usize);

        let artwork_pixels = artwork
            .argb8888_pixels()
            .expect("Didn't we call ensure_argb8888?");
        let terrain_pixels = match terrain.indexed_pixels() {
            Some(p) => p,
            None => {
                return Err(anyhow!(
                    "Level terrain image pixel format must be 8-bit indexed"
                ));
            }
        };

        let water_color = info
            .find_water_color(terrain.palette().unwrap(), SDL_PIXELFORMAT_ARGB8888)
            .unwrap_or(0xff0000ff);

        let snow_color = info.get_snow_color();

        // Copy level pixel data into the tile array
        let ts: usize = TILE_SIZE as usize;
        let src_pitch = tiles_wide as usize * ts;

        for j in 0..tiles_high as usize {
            for i in 0..tiles_wide as usize {
                let mut tile = TerrainTile {
                    terrain: [0; TILE_LENGTH],
                    artwork: [0; TILE_LENGTH],
                    content_hint: TileContentHint::Destructible,
                };

                for k in 0..ts {
                    let srcoffset = (j * ts + k) * src_pitch + i * ts;
                    let destoffset = k * ts;

                    tile.artwork[destoffset..destoffset + ts]
                        .clone_from_slice(&artwork_pixels[srcoffset..srcoffset + ts]);

                    tile.terrain[destoffset..destoffset + ts]
                        .clone_from_slice(&terrain_pixels[srcoffset..srcoffset + ts]);
                }

                if let Some(transparent_color_index) = transparent_color_index {
                    tile.artwork
                        .iter_mut()
                        .zip(tile.terrain.iter())
                        .for_each(|(a, t)| {
                            if *t == transparent_color_index {
                                *a = 0;
                            }
                        });
                }
                info.map_palette(&mut tile.terrain);
                tile.reset_content_hint();
                tiles.push(tile);
            }
        }

        // Initialize level texture. This will be updated when level is modified
        let mut artwork = Texture::new_streaming(renderer, artwork.width(), artwork.height())?;

        for i in 0..tiles_wide {
            for j in 0..tiles_high {
                let tile = &tiles[(j * tiles_wide + i) as usize];
                artwork.write_pixels(
                    &tile.artwork,
                    i * TILE_SIZE,
                    j * TILE_SIZE,
                    TILE_SIZE,
                    TILE_SIZE,
                );
            }
        }

        artwork.set_scalemode(TextureScaleMode::Nearest);

        Ok(Level {
            tiles,
            artwork,
            background,
            width,
            height,
            tiles_wide,
            tiles_high,
            forcefields: Vec::new(),
            water_color,
            snow_color,
        })
    }

    /// Level width in world coordinates
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Level height in world coordinates
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Return the terrain type at the given coordinates
    /// If out of bounds, will return the special "level bounds" terrain type
    pub fn terrain_at(&self, pos: Vec2) -> terrain::Terrain {
        let (x, y) = (pos.0, pos.1);
        if x < 0.0 || y < 0.0 || x >= self.width || y >= self.height {
            return terrain::TER_LEVELBOUND;
        }

        let x = (x / LEVEL_SCALE) as i32;
        let y = (y / LEVEL_SCALE) as i32;

        let xq = x / TILE_SIZE;
        let xr = x % TILE_SIZE;
        let yq = y / TILE_SIZE;
        let yr = y % TILE_SIZE;

        self.tiles[(yq * self.tiles_wide + xq) as usize].terrain[(yr * TILE_SIZE + xr) as usize]
    }

    /**
     * Check for collisions with solid terrain on the given line.
     *
     * Returns either the first point in which solid terrain was found,
     * or just the terrain type at the end if it was non-solid.
     */
    pub fn terrain_line(&self, line: LineF) -> Either<(terrain::Terrain, Vec2), terrain::Terrain> {
        if line.0.0 < 0.0 || line.0.1 < 0.0 || line.0.0 >= self.width || line.0.1 >= self.height {
            return Either::Left((
                terrain::TER_LEVELBOUND,
                Vec2(
                    line.1.0.clamp(0.0, self.width - 1.0),
                    line.1.1.clamp(0.0, self.height - 1.0),
                ),
            ));
        }

        let delta = (line.1 - line.0) / LEVEL_SCALE;
        if delta.magnitude_squared() <= 1.0 {
            let t = self.terrain_at(line.1);
            return if terrain::is_solid(t) {
                Either::Left((t, line.1))
            } else {
                Either::Right(t)
            };
        }

        let l = Line::from(line / LEVEL_SCALE);

        let mut i = l.x1 / TILE_SIZE;
        let mut j = l.y1 / TILE_SIZE;
        let last_i = l.x2 / TILE_SIZE;
        let last_j = l.y2 / TILE_SIZE;
        let mut prev_i = -1;
        let mut prev_j = -1;

        let mut last_non_solid: terrain::Terrain = 0;
        let mut loop_limit = 100;
        while loop_limit > 0 && (i != prev_i || j != prev_j) {
            prev_i = i;
            prev_j = j;
            loop_limit -= 1;
            let tile_rect = Rect::new(i * TILE_SIZE, j * TILE_SIZE, TILE_SIZE, TILE_SIZE);

            if let Some(isect) = l.intersected(&tile_rect) {
                let tile = &self.tiles[(j * self.tiles_wide + i) as usize];
                if let TileContentHint::FreeSpace = tile.content_hint {
                    last_non_solid = 0;
                } else if let TileContentHint::Water = tile.content_hint {
                    last_non_solid = TER_BIT_WATER;
                } else {
                    match tile.terrain_line(isect.offset(-tile_rect.x(), -tile_rect.y())) {
                        Either::Left((t, x, y)) => {
                            return Either::Left((
                                t,
                                Vec2(
                                    (x + tile_rect.x()) as f32 * LEVEL_SCALE,
                                    (y + tile_rect.y()) as f32 * LEVEL_SCALE,
                                ),
                            ));
                        }
                        Either::Right(t) => {
                            last_non_solid = t;
                        }
                    }
                }

                if i == last_i && j == last_j {
                    break;
                }

                if isect.x2 == tile_rect.x() && i > 0 && l.x1 > l.x2 {
                    i -= 1;
                } else if isect.x2 == tile_rect.right() && i < self.tiles_wide - 1 && l.x1 < l.x2 {
                    i += 1;
                }

                if isect.y2 == tile_rect.y() && j > 0 && l.y1 > l.y2 {
                    j -= 1;
                } else if isect.y2 == tile_rect.bottom() && j < self.tiles_high - 1 && l.y1 < l.y2 {
                    j += 1;
                }
            } else {
                break;
            }
        }

        if loop_limit == 0 {
            error!("Endless loop in terrain_line detected! (line: {})", l);
        }

        if line.1.0 < 0.0 || line.1.1 < 0.0 || line.1.0 >= self.width || line.1.1 >= self.height {
            return Either::Left((
                terrain::TER_LEVELBOUND,
                Vec2(
                    line.1.0.clamp(0.0, self.width - 1.0),
                    line.1.1.clamp(0.0, self.height - 1.0),
                ),
            ));
        }

        Either::Right(last_non_solid)
    }

    /// Remove the force field with the given ID
    pub fn remove_forcefield(&mut self, id: i32) {
        if let Some(idx) = self.forcefields.iter().position(|f| f.id == id) {
            self.forcefields.swap_remove(idx);
        }
    }

    /// Update a force field. If the field does not exist yet, it will be created
    pub fn update_forcefield(&mut self, ff: &Forcefield) {
        for f in self.forcefields.iter_mut() {
            if f.id == ff.id {
                *f = ff.clone();
                return;
            }
        }

        self.forcefields.push(ff.clone());
    }

    /// Return a rectangle centered on the given point and clamped to the level bounds
    pub fn camera_rect(&self, center: Vec2, width: f32, height: f32) -> RectF {
        RectF::new(
            (center.0 - width / 2.0).clamp(0.0, self.width - width),
            (center.1 - height / 2.0).clamp(0.0, self.height - height),
            width,
            height,
        )
    }

    /// Repaint tile artwork to the artwork texture
    pub(super) fn repaint_tile(&mut self, i: i32, j: i32) {
        let tile = &self.tiles[(j * self.tiles_wide + i) as usize];
        self.artwork.write_pixels(
            &tile.artwork,
            i * TILE_SIZE,
            j * TILE_SIZE,
            TILE_SIZE,
            TILE_SIZE,
        );
    }
    /**
     * Return a mutable tile iterator to the tiles intersecting the given rect (in unscaled coordinates)
     */
    fn tile_iterator(&self, rect: Rect) -> TileIterator<'_, TerrainTile> {
        let w = self.tiles_wide - 1;
        let h = self.tiles_high - 1;

        let tx0 = (rect.x() / TILE_SIZE).clamp(0, w);
        let tx1 = (rect.right() / TILE_SIZE).clamp(0, w);
        let ty0 = (rect.y() / TILE_SIZE).clamp(0, h);
        let ty1 = (rect.bottom() / TILE_SIZE).clamp(0, h);

        TileIterator::new(
            &self.tiles,
            self.tiles_wide as usize,
            tx0 as usize,
            ty0 as usize,
            (tx1 - tx0 + 1) as usize,
            (ty1 - ty0 + 1) as usize,
        )
    }

    /**
     * Return a mutable tile iterator to the tiles intersecting the given rect (in unscaled coordinates)
     */
    pub(super) fn tile_iterator_mut(&mut self, rect: Rect) -> MutableTileIterator<'_, TerrainTile> {
        let w = self.tiles_wide - 1;
        let h = self.tiles_high - 1;

        let tx0 = (rect.x() / TILE_SIZE).clamp(0, w);
        let tx1 = (rect.right() / TILE_SIZE).clamp(0, w);
        let ty0 = (rect.y() / TILE_SIZE).clamp(0, h);
        let ty1 = (rect.bottom() / TILE_SIZE).clamp(0, h);

        MutableTileIterator::new(
            &mut self.tiles,
            self.tiles_wide as usize,
            tx0 as usize,
            ty0 as usize,
            (tx1 - tx0 + 1) as usize,
            (ty1 - ty0 + 1) as usize,
        )
    }

    pub(super) fn tile_mut(&mut self, tx: i32, ty: i32) -> &mut TerrainTile {
        debug_assert!(tx >= 0 && ty >= 0);
        &mut self.tiles[(ty * self.tiles_wide + tx) as usize]
    }

    /**
     * Find a spawn point within the level, optionally constrained to the given area.
     *
     * If no spawnpoint is found after a reasonable number of tries, Error is returned.
     */
    pub fn find_spawnpoint(&self, in_area: Option<RectF>, allow_water: bool) -> Result<Vec2> {
        let in_area = match in_area {
            Some(r) => r,
            None => RectF::new(0.0, 0.0, self.width, self.height),
        };

        for _ in 0..100 {
            let pos = Vec2(
                in_area.x() + fastrand::f32() * in_area.w(),
                in_area.y() + fastrand::f32() * in_area.h(),
            );

            let ter = self.terrain_at(pos);
            if !terrain::is_solid(ter) && (allow_water || !terrain::is_underwater(ter)) {
                return Ok(pos);
            }
        }

        Err(anyhow!("Couldn't find a spawnpoint after 100 tries!"))
    }

    /// Render the level using the given camera rectangle
    pub fn render(&self, renderer: &Renderer, camera: RectF) {
        let source = RectF::new(
            camera.x() / LEVEL_SCALE,
            camera.y() / LEVEL_SCALE,
            camera.w() / LEVEL_SCALE,
            camera.h() / LEVEL_SCALE,
        );

        if let Some(bg) = self.background.as_ref() {
            bg.render_simple(
                renderer,
                Some(RectF::new(
                    (camera.x() / (self.width - camera.w())) * (bg.width() - camera.w()),
                    (camera.y() / (self.height - camera.h())) * (bg.height() - camera.h()),
                    camera.w(),
                    camera.h(),
                )),
                None,
            );
        }

        self.artwork.render_simple(renderer, Some(source), None);
    }

    pub fn debug_render_tilehints(&self, renderer: &Renderer, camera: RectF) {
        let source = Rect::new(
            (camera.x() / LEVEL_SCALE) as i32,
            (camera.y() / LEVEL_SCALE) as i32,
            (camera.w() / LEVEL_SCALE) as i32,
            (camera.h() / LEVEL_SCALE) as i32,
        );

        for (i, j, t) in self.tile_iterator(source) {
            let tr = RectF::new(
                (i * TILE_SIZE) as f32 * LEVEL_SCALE,
                (j * TILE_SIZE) as f32 * LEVEL_SCALE,
                TILE_SIZE as f32 * LEVEL_SCALE,
                TILE_SIZE as f32 * LEVEL_SCALE,
            );
            let color = match t.content_hint {
                TileContentHint::FreeSpace => Color::new_rgba(1.0, 1.0, 1.0, 0.1),
                TileContentHint::Water => Color::new_rgba(1.0, 0.0, 1.0, 0.8),
                TileContentHint::Destructible => Color::new_rgba(1.0, 0.0, 0.0, 0.4),
                TileContentHint::Indestructible => Color::new_rgba(0.0, 1.0, 1.0, 0.4),
            };
            renderer.draw_filled_rectangle(tr.offset(-camera.x(), -camera.y()), &color);
        }
    }
}

impl TerrainTile {
    pub(super) fn reset_content_hint(&mut self) {
        let mut all_free_space = true;
        let mut all_water = true;
        for t in self.terrain {
            if terrain::is_solid(t) {
                all_free_space = false;
                all_water = false;
                if terrain::is_destructible(t) {
                    self.content_hint = TileContentHint::Destructible;
                    return;
                }
            } else if t != TER_BIT_WATER {
                all_water = false;
            } else if t != 0 {
                all_free_space = false;
            }
        }

        self.content_hint = if all_water {
            TileContentHint::Water
        } else if all_free_space {
            TileContentHint::FreeSpace
        } else {
            // no destructable solids, no free space
            TileContentHint::Indestructible
        };
    }

    fn terrain_line(&self, line: Line) -> Either<(terrain::Terrain, i32, i32), terrain::Terrain> {
        debug_assert!(line.x1 >= 0 && line.x1 < TILE_SIZE);
        debug_assert!(line.x2 >= 0 && line.x2 < TILE_SIZE);
        debug_assert!(line.y1 >= 0 && line.y1 < TILE_SIZE);
        debug_assert!(line.y2 >= 0 && line.y2 < TILE_SIZE);

        // Bresenham's line algorithm
        let dx = (line.x2 - line.x1).abs();
        let dy = -(line.y2 - line.y1).abs();
        let sx = if line.x1 < line.x2 { 1 } else { -1 };
        let sy = if line.y1 < line.y2 { 1 } else { -1 };

        let mut error = dx + dy;
        let mut x = line.x1;
        let mut y = line.y1;
        loop {
            let ter = self.terrain[(y * TILE_SIZE + x) as usize];
            if terrain::is_solid(ter) {
                return Either::Left((ter, x, y));
            }
            let e2 = error * 2;
            if e2 >= dy {
                if x == line.x2 {
                    break;
                }
                error = error + dy;
                x += sx;
            }
            if e2 <= dx {
                if y == line.y2 {
                    break;
                }
                error = error + dx;
                y += sy;
            }
        }

        Either::Right(self.terrain[(line.y2 * TILE_SIZE + line.x2) as usize])
    }
}
