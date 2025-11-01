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

use super::PhysicalObject;
use crate::{
    game::{
        level::{
            LEVEL_SCALE, Level,
            terrain::{self, Terrain},
        },
        objects::{GameObject, TerrainCollisionMode},
    },
    gfx::{Color, RenderDest, RenderOptions, Renderer, TextureId},
    math::{RectF, Vec2},
};

/**
 * Dust, snow, or other type of atomized terrain that floats down.
 *
 * Upon touching ground, terrain particles can turn into actual terrain pixels.
 */
#[derive(Clone, Debug)]
pub struct TerrainParticle {
    phys: PhysicalObject,
    texture: Option<TextureId>,
    color: Color,
    terrain: Terrain, // if zero, this particle won't turn into real terrain
    destroyed: bool,
}

impl mlua::FromLua for TerrainParticle {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            Ok(TerrainParticle {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get("vel")?,
                    imass: 100.0,
                    radius: LEVEL_SCALE / 2.0,
                    drag: 0.3,
                    impulse: Vec2::ZERO,
                    terrain_collision_mode: TerrainCollisionMode::Simple,
                },
                texture: table.get("texture")?,
                terrain: table.get::<Option<Terrain>>("terrain")?.unwrap_or(0),
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                destroyed: false,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "TerrainParticle".to_owned(),
                message: Some("expected a table describing a terrain particle".to_string()),
            })
        }
    }
}

impl TerrainParticle {
    pub fn new(pos: Vec2, terrain: Terrain, texture: Option<TextureId>, color: Color) -> Self {
        TerrainParticle {
            phys: PhysicalObject {
                pos,
                vel: Vec2::ZERO,
                imass: 100.0,
                radius: LEVEL_SCALE / 2.0,
                drag: 0.3,
                impulse: Vec2::ZERO,
                terrain_collision_mode: TerrainCollisionMode::Simple,
            },
            texture,
            terrain,
            color,
            destroyed: false,
        }
    }

    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn physics_mut(&mut self) -> &mut PhysicalObject {
        &mut self.phys
    }

    pub fn step_mut(
        &mut self,
        level: &Level,
        windspeed: f32,
        timestep: f32,
    ) -> Option<(Vec2, Terrain, Color)> {
        let jitter = -1000.0 + fastrand::f32() * 2000.0;
        self.phys.vel.0 += (windspeed + jitter) * timestep;
        let ter = self.phys.step(level, timestep);

        if ter != 0 {
            self.destroyed = true;

            if terrain::is_level_boundary(ter) {
                return None;
            }

            if terrain::is_water(ter) && !terrain::is_ice(self.terrain) {
                // Only ice floats
                return None;
            }
            return Some((self.pos(), self.terrain, self.color));
        }

        None
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        if let Some(tex) = self.texture {
            renderer.texture_store().get_texture(tex).render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(self.phys.pos - camera_pos),
                    color: self.color,
                    ..Default::default()
                },
            );
        } else {
            let p = self.pos() - camera_pos;
            renderer
                .draw_filled_rectangle(RectF::new(p.0, p.1, LEVEL_SCALE, LEVEL_SCALE), &self.color);
        }
    }
}

impl GameObject for TerrainParticle {
    fn pos(&self) -> Vec2 {
        self.phys.pos
    }

    fn radius(&self) -> f32 {
        LEVEL_SCALE / 2.0
    }

    fn is_destroyed(&self) -> bool {
        self.destroyed
    }
}
