// This file is part of Luola2
// Copyright (C) 2025, 2026 Calle Laakkonen
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
    gfx::{Color, RenderDest, RenderOptions, Renderer, TextureId, TextureStore},
    math::{RectF, Vec2},
};
use anyhow::anyhow;
use core::ops::Deref;

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
    lifetime: f32,
    wind: bool, // does wind and jitter affect this particle
    ground_interaction: GroundInteractionStyle,
    destroyed: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum GroundInteractionStyle {
    Decorative,    // simply disappears when touching ground
    Staining,      // changes the color of existing ground but doesn't add more
    Ember,         // burning particle that can ignite burnable or explosive terrain
    Sand(Terrain), // turns to more ground (falling sand if enabled)
}

impl mlua::FromLua for GroundInteractionStyle {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Nil => Ok(Self::Decorative),
            mlua::Value::Integer(0) => Ok(Self::Decorative),
            mlua::Value::Integer(i) if i > 0 && i < 256 => Ok(Self::Sand(i as u8)),
            mlua::Value::Integer(_) => Err(anyhow!("value must be in range 0..255").into()),
            mlua::Value::String(s) => match s.as_bytes().deref() {
                b"decorative" => Ok(Self::Decorative),
                b"staining" => Ok(Self::Staining),
                b"ember" => Ok(Self::Ember),
                b"ground" => Ok(Self::Sand(
                    terrain::TER_TYPE_GROUND | terrain::TER_BIT_DESTRUCTIBLE,
                )),
                b"ice" => Ok(Self::Sand(
                    terrain::TER_TYPE_ICE | terrain::TER_BIT_DESTRUCTIBLE,
                )),
                unknown => {
                    //  TODO parse terrain name
                    Err(anyhow!("Unknown mode: {}", str::from_utf8(unknown).unwrap()).into())
                }
            },
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "GroundInteractionStyle".to_owned(),
                message: Some("expected GroundInteractionStyle".to_string()),
            }),
        }
    }
}

impl mlua::FromLua for TerrainParticle {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let ground_interaction = table.get::<GroundInteractionStyle>("ground_interaction")?;
            Ok(TerrainParticle {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get("vel")?,
                    imass: 100.0,
                    radius: LEVEL_SCALE / 2.0,
                    drag: table.get::<Option<f32>>("drag")?.unwrap_or(0.6),
                    impulse: Vec2::ZERO,
                    terrain_collision_mode: match ground_interaction {
                        GroundInteractionStyle::Sand(_) => TerrainCollisionMode::Simple,
                        _ => TerrainCollisionMode::Passthrough,
                    },
                    antigrav: false,
                },
                texture: table.get("texture")?,
                ground_interaction,
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                wind: table.get::<Option<bool>>("wind")?.unwrap_or(true),
                lifetime: table.get::<Option<f32>>("lifetime")?.unwrap_or(60.6),
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
    pub fn new(
        pos: Vec2,
        ground_interaction: GroundInteractionStyle,
        texture: Option<TextureId>,
        color: Color,
        zerogravity: bool,
    ) -> Self {
        TerrainParticle {
            phys: PhysicalObject {
                pos,
                vel: if zerogravity {
                    Vec2::for_rad(
                        fastrand::f32() * 2.0 * std::f32::consts::PI,
                        fastrand::f32() * 100.0 + 50.0,
                    )
                } else {
                    Vec2::ZERO
                },
                imass: 100.0,
                radius: LEVEL_SCALE / 2.0,
                drag: 0.3,
                impulse: Vec2::ZERO,
                terrain_collision_mode: TerrainCollisionMode::Simple,
                antigrav: false,
            },
            lifetime: 60.0,
            texture,
            color,
            ground_interaction,
            wind: !zerogravity,
            destroyed: false,
        }
    }

    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn physics_mut(&mut self) -> &mut PhysicalObject {
        &mut self.phys
    }

    pub fn ground_interaction(&self) -> GroundInteractionStyle {
        self.ground_interaction
    }

    pub fn is_staining(&self) -> bool {
        matches!(self.ground_interaction, GroundInteractionStyle::Staining)
    }

    /// Get the terrain type of this particle, or 0 if it does not leave any terrain behind
    pub fn terrain(&self) -> Terrain {
        match self.ground_interaction {
            GroundInteractionStyle::Sand(t) => t,
            _ => 0,
        }
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn step_mut(&mut self, level: &Level, timestep: f32) -> Option<Vec2> {
        if self.wind {
            let jitter = (-0.5 + fastrand::f32()) * 0.5;
            self.phys
                .add_impulse(Vec2(level.windspeed() / 10.0 + jitter, 0.0));
        }

        let (_, ter) = self.phys.step(level, timestep);

        if !terrain::is_space(ter) {
            self.destroyed = true;

            if !matches!(self.ground_interaction, GroundInteractionStyle::Decorative) {
                if terrain::is_level_boundary(ter) || terrain::is_effective_base(ter) {
                    // Don't accumulate on level boundaries or bases.
                    // Bases we don't want covered because it could make them easily unusable by
                    // accident. (Lore explanation: the pit crew keeps cleaning them up.)
                    return None;
                }

                if terrain::is_water(ter) && !terrain::is_ice(self.terrain()) {
                    // Only ice floats
                    return None;
                }

                return Some(self.pos());
            }
        } else if matches!(self.ground_interaction, GroundInteractionStyle::Ember) {
            // animate color
            //self.color = Color::from_hsv((30.0 - self.lifetime) * 2.0, 1.0, 1.0);
            self.color = Color::from_hsv(fastrand::f32() * 60.0, 1.0, 1.0)
                .with_alpha(self.lifetime.min(1.0));
        }

        self.lifetime -= timestep;
        if self.lifetime <= 0.0 {
            self.destroyed = true;
        }

        None
    }

    pub fn render(&self, renderer: &Renderer, ts: &TextureStore, camera_pos: Vec2) {
        if let Some(tex) = self.texture {
            ts.get_texture(tex).render(
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
