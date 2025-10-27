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
    game::level::{
        LEVEL_SCALE, TILE_SIZE, TileContentHint, rectiter::MutableRectIterator, terrain,
    },
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

    /**
     * Make a regular bullet hole
     */
    pub fn make_standard_bullet_hole(&mut self, pos: Vec2) {
        self.make_hole(pos, 3);
    }

    /**
     * Make an arbitrarily sized hole
     */
    pub fn make_hole(&mut self, pos: Vec2, r: i32) {
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

                    for (ter, art) in terrain_row.into_iter().zip(art_row.into_iter()) {
                        let dd = dy * dy + dx * dx;
                        if dd <= rr && terrain::is_destructible(*ter) {
                            if terrain::is_underwater(*ter) {
                                *art = water_color;
                            } else {
                                *art = 0;
                            }

                            *ter &= !terrain::TER_MASK_SOLID;
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

    /**
     * Update dirtied texture tiles (if any)
     */
    pub fn apply_texture_changes(&mut self) {
        for (i, j) in self.dirty_set.drain() {
            // TODO update content hint?
            self.level.repaint_tile(i, j);
        }
    }
}

impl<'a> Drop for LevelEditor<'a> {
    fn drop(&mut self) {
        self.apply_texture_changes();
    }
}
