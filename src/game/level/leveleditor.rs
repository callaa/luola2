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
            Forcefield, LEVEL_SCALE, RegeneratingTerrain, TILE_SIZE, TileContentHint,
            dynter::{DynamicTerrainCell, DynamicTerrainMap},
            rectiter::MutableRectIterator,
            terrain::{
                self, TER_BIT_DESTRUCTIBLE, TER_BIT_DYNAMIC, TER_BIT_WATER, TER_MASK_SOLID,
                TER_TYPE_DAMAGE, TER_TYPE_GREYGOO, TER_TYPE_GROUND, TER_TYPE_HIGH_EXPLOSIVE,
                TER_TYPE_ICE, Terrain,
            },
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
    water_color: u32,
    snow_color: u32,
    dirty_set: HashSet<(i32, i32)>,
}

impl<'a> LevelEditor<'a> {
    pub fn new(level: &'a mut Level) -> Self {
        let water_color = level.water_color;
        let snow_color = level.snow_color;
        Self {
            level,
            water_color,
            snow_color,
            dirty_set: HashSet::new(),
        }
    }

    pub fn update_forcefield(&mut self, ff: &Forcefield) {
        self.level.update_forcefield(ff);
    }

    pub fn remove_forcefield(&mut self, id: i32) {
        self.level.remove_forcefield(id);
    }

    pub fn set_windspeed(&mut self, ws: f32) {
        self.level.set_windspeed(ws);
    }

    /**
     * Make a regular bullet hole
     */
    pub fn make_standard_bullet_hole(&mut self, pos: Vec2, scripting: &mut ScriptEnvironment) {
        self.make_hole(pos, 3, 0.05, scripting);
    }

    /**
     * Make an arbitrarily sized hole
     */
    pub fn make_hole(
        &mut self,
        pos: Vec2,
        r: i32,
        dust_chance: f32,
        scripting: &mut ScriptEnvironment,
    ) {
        if r <= 0 {
            return;
        }

        let center_x = (pos.0 / LEVEL_SCALE) as i32;
        let center_y = (pos.1 / LEVEL_SCALE) as i32;
        let rr = r * r;

        let hole_rect = Rect::new(center_x - r, center_y - r, r * 2, r * 2);

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
                        terrain_row.iter_mut().zip(art_row.iter_mut()).enumerate()
                    {
                        let dd = dy * dy + dx * dx;
                        if dd <= rr && terrain::is_destructible(*ter) {
                            if (terrain::is_high_explosive(*ter) && fastrand::f32() < 0.3)
                                || (terrain::is_explosive(*ter) && fastrand::f32() < 0.05)
                            {
                                match scripting.get_function("luola_explosive_terrain") {
                                    Ok(f) => {
                                        if let Err(err) = f.call::<()>((
                                            Vec2(
                                                (tile_rect.x() + rect_in_tile.x() + row_x as i32)
                                                    as f32
                                                    * LEVEL_SCALE,
                                                (tile_rect.y() + y as i32) as f32 * LEVEL_SCALE,
                                            ),
                                            *art,
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
                            } else if terrain::is_dynamic(*ter) {
                                // This is called for destroyed terrain because
                                // LooseningSand will spread to adjacent non-destroyed pixels
                                scripting.add_effect(WorldEffect::AddDynamicTerrain(
                                    Vec2(
                                        (tile_rect.x() + rect_in_tile.x() + row_x as i32) as f32
                                            * LEVEL_SCALE,
                                        (tile_rect.y() + y as i32) as f32 * LEVEL_SCALE,
                                    ),
                                    DynamicTerrainCell::LooseningSand,
                                ));
                            } else if fastrand::f32() < dust_chance {
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
                                *art = self.water_color;
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

    // Replace a point with a solid or an empty space.
    // underwater bit is preserved.
    fn replace_point_lc(&mut self, pos: (i32, i32), solid: Terrain, color: u32) {
        debug_assert!((solid & !(TER_MASK_SOLID | TER_BIT_DYNAMIC)) == 0);

        if let Some((tile, offset, tilepos)) = self.level.tile_at_lc_mut(pos) {
            if solid == 0 {
                tile.terrain[offset] &= !(TER_BIT_DESTRUCTIBLE | TER_MASK_SOLID | TER_BIT_DYNAMIC);
                if terrain::is_underwater(tile.terrain[offset]) {
                    tile.artwork[offset] = self.water_color;
                } else {
                    tile.artwork[offset] = color;
                }
            } else {
                if terrain::is_underwater(tile.terrain[offset]) {
                    tile.artwork[offset] = Color::from_argb_u32(color)
                        .blend(Color::from_argb_u32(self.water_color).with_alpha(0.5))
                        .as_argb_u32();
                } else if color & 0xff000000 != 0xff000000 {
                    tile.artwork[offset] = Color::from_argb_u32(tile.artwork[offset])
                        .blend(Color::from_argb_u32(color))
                        .as_argb_u32();
                } else {
                    tile.artwork[offset] = color;
                }
                tile.terrain[offset] =
                    (tile.terrain[offset] & !TER_MASK_SOLID) | TER_BIT_DESTRUCTIBLE | solid;
            };
            self.dirty_set.insert(tilepos);
        }
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

    /// Add a new dynamic terrain cell
    pub fn add_dynterrain(&mut self, pos: Vec2, dter: DynamicTerrainCell) {
        if !dter.destroys_ground() || !terrain::is_indestructible_solid(self.level.terrain_at(pos))
        {
            let mut cells = self.level.dynterrain.take();
            cells.insert(
                ((pos.0 / LEVEL_SCALE) as i32, (pos.1 / LEVEL_SCALE) as i32),
                dter,
            );

            self.level.dynterrain.replace(cells);
        }
    }

    #[inline]
    fn neighbors(n: &[(i32, i32)], pos: (i32, i32)) -> impl Iterator<Item = (i32, i32)> {
        n.iter().map(move |&p| (p.0 + pos.0, p.1 + pos.1))
    }

    /// Perform a dynamic terrain simulation step
    /// Note: this assumes a fixed timestep of 60FPS
    pub fn step_dynterrain(&mut self) {
        let old_cells = self.level.dynterrain.take();
        if old_cells.is_empty() {
            return;
        }

        let mut new_cells = DynamicTerrainMap::new();

        'outer: for (&pos, &cell) in old_cells.iter() {
            match cell {
                DynamicTerrainCell::Foam { limit } => {
                    self.replace_point_lc(
                        pos,
                        TER_TYPE_GROUND,
                        Color::from_hsv(
                            37.0 - limit as f32 / 2.0,
                            0.372,
                            0.811 - (limit as f32 / (30.0 * 3.0)) + fastrand::f32() * 0.1 - 0.05,
                        )
                        .as_argb_u32(),
                    );
                    if limit > 0 {
                        Self::neighbors(&NEIGHBORS_ROUNDISH, pos).for_each(|p| {
                            if !terrain::is_solid(self.level.terrain_at_lc(p))
                                && !new_cells.contains_key(&p)
                                && fastrand::f32() * limit as f32 > 3.0
                            {
                                new_cells.insert(p, DynamicTerrainCell::Foam { limit: limit - 1 });
                            }
                        });
                    }
                }
                DynamicTerrainCell::GreyGoo { counter, limit } => {
                    if counter > 0 {
                        self.replace_point_lc(
                            pos,
                            TER_TYPE_GREYGOO,
                            Color::from_hsv(183.0, 1.0, counter as f32 / 6.0).as_argb_u32(),
                        );
                        new_cells.insert(
                            pos,
                            DynamicTerrainCell::GreyGoo {
                                counter: counter - 1,
                                limit,
                            },
                        );
                    } else {
                        self.replace_point_lc(pos, 0, 0);
                        if limit > 0 {
                            Self::neighbors(&NEIGHBORS4, pos).for_each(|p| {
                                let tp = self.level.terrain_at_lc(p);
                                if terrain::is_destructible(tp) && !terrain::is_greygoo(tp) {
                                    new_cells.insert(
                                        p,
                                        DynamicTerrainCell::GreyGoo {
                                            counter: fastrand::i32(20..40),
                                            limit: limit - 1,
                                        },
                                    );
                                }
                            });
                        }
                    }
                }
                DynamicTerrainCell::Nitro { counter, limit } => {
                    if counter > 0 {
                        new_cells.insert(
                            pos,
                            DynamicTerrainCell::Nitro {
                                counter: counter - 1,
                                limit,
                            },
                        );
                    } else {
                        self.replace_point_lc(pos, TER_TYPE_HIGH_EXPLOSIVE, 0xffff0000);
                        if limit > 0 {
                            Self::neighbors(&NEIGHBORS_ROUNDISH, pos).for_each(|p| {
                                let ter_at_p = self.level.terrain_at_lc(p);
                                if terrain::is_destructible(ter_at_p)
                                    && !terrain::is_high_explosive(ter_at_p)
                                    && !new_cells.contains_key(&p)
                                {
                                    new_cells.insert(
                                        p,
                                        DynamicTerrainCell::Nitro {
                                            counter: fastrand::i32(1..6),
                                            limit: limit - 1,
                                        },
                                    );
                                }
                            });
                        }
                    }
                }
                DynamicTerrainCell::Fire { counter, cinder } => {
                    if counter == 30 {
                        Self::neighbors(&NEIGHBORS_ROUNDISH, pos).for_each(|p| {
                            let ter_at_p = self.level.terrain_at_lc(p);
                            if terrain::is_burnable(ter_at_p) && !new_cells.contains_key(&p) {
                                new_cells.insert(
                                    p,
                                    DynamicTerrainCell::Fire {
                                        counter: fastrand::i32(31..60),
                                        cinder: terrain::is_cinder(ter_at_p),
                                    },
                                );
                            }
                        });
                    }

                    if counter > 0 {
                        self.replace_point_lc(
                            pos,
                            TER_TYPE_DAMAGE,
                            Color::from_hsv((60 - counter) as f32, 1.0, 1.0).as_argb_u32(),
                        );
                        new_cells.insert(
                            pos,
                            DynamicTerrainCell::Fire {
                                counter: counter - 1,
                                cinder,
                            },
                        );
                    } else if cinder {
                        let shade = fastrand::f32() * 0.1 + 0.2;
                        self.replace_point_lc(
                            pos,
                            TER_TYPE_GROUND,
                            Color::new(shade, shade, shade).as_argb_u32(),
                        );
                    } else {
                        self.replace_point_lc(pos, 0, 0);
                    }
                }
                DynamicTerrainCell::Freezer { limit } => {
                    if !terrain::is_solid(self.level.terrain_at_lc(pos)) {
                        self.replace_point_lc(pos, TER_TYPE_ICE, self.snow_color);
                    }
                    if limit > 0 {
                        Self::neighbors(&NEIGHBORS_ROUNDISH, pos).for_each(|p| {
                            let ter_at_p = self.level.terrain_at_lc(p);

                            if terrain::is_space(ter_at_p) {
                                // In open air, ice spreads as a thin surface hugging layer
                                let neighboring_solid = Self::neighbors(&NEIGHBORS8, p).any(|n| {
                                    let ter_at_n = self.level.terrain_at_lc(n);
                                    terrain::is_solid(ter_at_n) && !terrain::is_ice(ter_at_n)
                                });

                                if neighboring_solid {
                                    new_cells.insert(
                                        p,
                                        DynamicTerrainCell::Freezer { limit: limit - 1 },
                                    );

                                    if fastrand::f32() < 0.05 {
                                        // icicles
                                        for y in 1..4 {
                                            let icepos = (pos.0, pos.1 + y);
                                            if terrain::is_space(self.level.terrain_at_lc(icepos)) {
                                                self.replace_point_lc(
                                                    icepos,
                                                    TER_TYPE_ICE,
                                                    self.snow_color,
                                                );
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                            } else if terrain::is_water(ter_at_p) {
                                // spreads in all directions underwater
                                if fastrand::f32() * limit as f32 > 3.0 {
                                    new_cells.insert(
                                        p,
                                        DynamicTerrainCell::Freezer { limit: limit - 3 },
                                    );
                                }
                            }
                        });
                    }
                }
                DynamicTerrainCell::Toxin { limit } => {
                    self.replace_point_lc(pos, TER_TYPE_DAMAGE, 0x60ff2b80);
                    if limit > 0 {
                        Self::neighbors(&NEIGHBORS4, pos).for_each(|p| {
                            let ter_at_p: u8 = self.level.terrain_at_lc(p);
                            if terrain::is_destructible(ter_at_p)
                                && !terrain::is_damaging(ter_at_p)
                                && !terrain::is_effective_base(ter_at_p)
                            {
                                let neighboring_space = Self::neighbors(&NEIGHBORS_ROUNDISH, p)
                                    .any(|n| terrain::is_space(self.level.terrain_at_lc(n)));
                                if neighboring_space {
                                    new_cells
                                        .insert(p, DynamicTerrainCell::Toxin { limit: limit - 1 });
                                }
                            }
                        });
                    }
                }
                DynamicTerrainCell::LooseningSand => {
                    let terrain = self.level.terrain_at_lc(pos);
                    if terrain::is_dynamic(terrain) && !new_cells.contains_key(&pos) {
                        new_cells.insert(
                            pos,
                            DynamicTerrainCell::Sand {
                                terrain: terrain & (TER_MASK_SOLID | TER_BIT_DYNAMIC),
                                solidify: 60 * 10,
                                color: self.level.pixel_at_lc(pos),
                            },
                        );
                    }

                    Self::neighbors(&NEIGHBORS8, pos).for_each(|p| {
                        let ter_at_p = self.level.terrain_at_lc(p);
                        if terrain::is_dynamic(ter_at_p)
                            && !new_cells.contains_key(&p)
                            && !old_cells.contains_key(&p)
                        {
                            new_cells.insert(p, DynamicTerrainCell::LooseningSand);
                        }
                    });
                }

                DynamicTerrainCell::Sand {
                    terrain,
                    solidify,
                    color,
                } => {
                    if solidify > 0
                        && self.level.terrain_at_lc(pos) & (TER_MASK_SOLID | TER_BIT_DYNAMIC)
                            == terrain
                    {
                        const FLOW: [(i32, i32); 5] = [(0, 1), (-1, 1), (1, 1), (-2, 1), (2, 1)];
                        for f in FLOW {
                            let p = (pos.0 + f.0, pos.1 + f.1);
                            if !terrain::is_solid(self.level.terrain_at_lc(p)) {
                                self.replace_point_lc(pos, 0, 0);
                                self.replace_point_lc(p, terrain, color);
                                new_cells.insert(
                                    p,
                                    DynamicTerrainCell::Sand {
                                        terrain,
                                        solidify,
                                        color,
                                    },
                                );
                                continue 'outer;
                            }
                        }
                        new_cells.insert(
                            pos,
                            DynamicTerrainCell::Sand {
                                terrain,
                                solidify: solidify - 1,
                                color,
                            },
                        );
                    }
                }
            }
        }

        // println!("DynTerrain {}", new_cells.len());

        self.level.dynterrain.replace(new_cells);
    }

    /**
     * Perform a terrain regeneration step.
     *
     * Returns true if at least one pixel was regenerated
     */
    pub fn regenerate_terrain(&mut self) -> bool {
        let first = self.level.regen.iter().position(|r| {
            !terrain::is_solid(self.level.tile(r.tile.0, r.tile.1).terrain[r.offset])
        });

        let mut changes: Vec<RegeneratingTerrain> = Vec::new();

        if let Some(first) = first {
            let row = self.level.regen[first].offset / TILE_SIZE as usize;
            changes.push(self.level.regen[first].clone());
            for r in &self.level.regen[(first + 1)..] {
                if (r.offset / TILE_SIZE as usize) != row {
                    break;
                }
                if !terrain::is_solid(self.level.tile(r.tile.0, r.tile.1).terrain[r.offset])
                    && ((terrain::is_basesupport(r.terrain) && fastrand::f32() < 0.7)
                        || fastrand::f32() < 0.3)
                {
                    changes.push(r.clone());
                }
            }
        }

        for r in changes.iter() {
            let tile = self.level.tile_mut(r.tile.0, r.tile.1);
            tile.artwork[r.offset] = r.color;
            tile.terrain[r.offset] = r.terrain;
            self.dirty_set.insert(r.tile);
        }

        !changes.is_empty()
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

/// Von Neumann neighborhood
static NEIGHBORS4: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Moore neighborhood
static NEIGHBORS8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (-1, 1),
    (1, 1),
];

static NEIGHBORS_ROUNDISH: [(i32, i32); 12] = [
    (0, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (-1, -2),
    (1, -2),
    (-2, -1),
    (2, -1),
    (-2, 1),
    (2, 1),
    (-1, 2),
    (1, 2),
];
impl<'a> Drop for LevelEditor<'a> {
    fn drop(&mut self) {
        self.apply_texture_changes();
    }
}
