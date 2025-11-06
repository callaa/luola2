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

use std::collections::HashSet;

use crate::{
    game::{
        level::{
            Forcefield, LEVEL_SCALE, TILE_SIZE, TileContentHint,
            rectiter::MutableRectIterator,
            terrain::{self, TER_BIT_WATER, Terrain},
        },
        objects::TerrainParticle,
        scripting::ScriptEnvironment,
        world::WorldEffect,
    },
    gfx::Color,
    math::{Rect, Vec2},
};

use super::level::Level;

pub struct LevelEditor<'a> {
    level: &'a mut Level,
    dirty_set: HashSet<(i32, i32)>,
}

impl<'a> LevelEditor<'a> {
    pub fn new(level: &'a mut Level) -> Self {
        Self {
            level,
            dirty_set: HashSet::new(),
        }
    }

    pub fn update_forcefield(&mut self, ff: &Forcefield) {
        self.level.update_forcefield(ff);
    }

    pub fn remove_forcefield(&mut self, id: i32) {
        self.level.remove_forcefield(id);
    }

    /**
     * Make a regular bullet hole
     */
    pub fn make_standard_bullet_hole(&mut self, pos: Vec2, scripting: &mut ScriptEnvironment) {
        self.make_hole(pos, 3, scripting);
    }

    /**
     * Make an arbitrarily sized hole
     */
    pub fn make_hole(&mut self, pos: Vec2, r: i32, scripting: &mut ScriptEnvironment) {
        if r <= 0 {
            return;
        }

        let center_x = (pos.0 / LEVEL_SCALE) as i32;
        let center_y = (pos.1 / LEVEL_SCALE) as i32;
        let rr = r * r;

        let hole_rect = Rect::new(center_x - r, center_y - r, r * 2, r * 2);

        let water_color = self.level.water_color;

        for (i, j, tile) in self.level.tile_iterator_mut(hole_rect) {
            if let TileContentHint::Destructible = tile.content_hint {
                let mut dirty = false;
                let tile_rect = Rect::new(i * TILE_SIZE, j * TILE_SIZE, TILE_SIZE, TILE_SIZE);
                let rect_in_tile = hole_rect
                    .intersected(tile_rect)
                    .unwrap()
                    .offset(-tile_rect.x(), -tile_rect.y());

                let terrain_iter = MutableRectIterator::from_rect(
                    &mut tile.terrain,
                    TILE_SIZE as usize,
                    &rect_in_tile,
                );

                let artwork_iter = MutableRectIterator::from_rect(
                    &mut tile.artwork,
                    TILE_SIZE as usize,
                    &rect_in_tile,
                );

                for ((terrain_row, y), (art_row, _)) in terrain_iter.zip(artwork_iter) {
                    let dy = (tile_rect.y() + y as i32) - center_y;
                    let mut dx = (tile_rect.x() + rect_in_tile.x()) - center_x;

                    for (row_x, (ter, art)) in
                        terrain_row.into_iter().zip(art_row.into_iter()).enumerate()
                    {
                        let dd = dy * dy + dx * dx;
                        if dd <= rr && terrain::is_destructible(*ter) {
                            if (terrain::is_high_explosive(*ter) && fastrand::f32() < 0.3)
                                || (terrain::is_explosive(*ter) && fastrand::f32() < 0.05)
                            {
                                match scripting.get_function("luola_explosive_terrain") {
                                    Ok(f) => {
                                        if let Err(err) = f.call::<()>((
                                            (tile_rect.x() + rect_in_tile.x() + row_x as i32)
                                                as f32
                                                * LEVEL_SCALE,
                                            (tile_rect.y() + y as i32) as f32 * LEVEL_SCALE,
                                        )) {
                                            log::error!(
                                                "Call to luola_explosive_terrain failed: {err}"
                                            );
                                        }
                                    }
                                    Err(err) => {
                                        log::error!(
                                            "Couldn't get luola_explosive_terrain function: {err}"
                                        );
                                    }
                                }
                            } else if fastrand::f32() < 0.05 {
                                // create dust
                                let pos = Vec2(
                                    (tile_rect.x() + rect_in_tile.x() + row_x as i32) as f32
                                        * LEVEL_SCALE,
                                    (tile_rect.y() + y as i32) as f32 * LEVEL_SCALE,
                                );

                                scripting.add_effect(WorldEffect::AddTerrainParticle(
                                    TerrainParticle::new(
                                        pos,
                                        *ter,
                                        None,
                                        Color::from_argb_u32(*art),
                                    ),
                                ));
                            }

                            if terrain::is_underwater(*ter) {
                                *art = water_color;
                            } else {
                                *art = 0;
                            }

                            *ter &= !(terrain::TER_MASK_SOLID | terrain::TER_BIT_DESTRUCTIBLE);
                            dirty = true;
                        }

                        dx += 1;
                    }
                }

                if dirty {
                    self.dirty_set.insert((i, j));
                }
            }
        }
    }

    /// Change the color of an artwork pixel without changing the terrain type
    pub fn color_point(&mut self, pos: Vec2, color: Color) {
        if pos.0 < 0.0 || pos.1 < 0.0 || pos.0 >= self.level.width() || pos.1 >= self.level.height()
        {
            return;
        }

        let x = (pos.0 / LEVEL_SCALE) as i32;
        let y = (pos.1 / LEVEL_SCALE) as i32;

        let i = x / TILE_SIZE;
        let j = y / TILE_SIZE;
        let tile = self.level.tile_mut(i, j);

        let offset = ((y - j * TILE_SIZE) * TILE_SIZE + (x - i * TILE_SIZE)) as usize;

        let pix = Color::from_argb_u32(tile.artwork[offset]);

        tile.artwork[offset] = pix.blend(color).as_argb_u32();
        self.dirty_set.insert((i, j));
    }

    /**
     * Add a terrain point. Change is performed only if there is not
     * already a solid pixel in the given position.
     */
    pub fn add_point(&mut self, pos: Vec2, ter: Terrain, color: Color) {
        if pos.0 < 0.0 || pos.1 < 0.0 || pos.0 >= self.level.width() || pos.1 >= self.level.height()
        {
            return;
        }

        let x = (pos.0 / LEVEL_SCALE) as i32;
        let y = (pos.1 / LEVEL_SCALE) as i32;

        let i = x / TILE_SIZE;
        let j = y / TILE_SIZE;
        let tile = self.level.tile_mut(i, j);

        let offset = ((y - j * TILE_SIZE) * TILE_SIZE + (x - i * TILE_SIZE)) as usize;

        if !terrain::is_solid(tile.terrain[offset]) {
            tile.terrain[offset] = if terrain::is_underwater(tile.terrain[offset]) {
                ter | TER_BIT_WATER
            } else {
                ter
            };
            tile.artwork[offset] = color.as_argb_u32();
            self.dirty_set.insert((i, j));
        }
    }

    /**
     * Update dirtied texture tiles (if any)
     */
    pub fn apply_texture_changes(&mut self) {
        for (i, j) in self.dirty_set.drain() {
            self.level.tile_mut(i, j).reset_content_hint();
            self.level.repaint_tile(i, j);
        }
    }
}

impl<'a> Drop for LevelEditor<'a> {
    fn drop(&mut self) {
        self.apply_texture_changes();
    }
}
