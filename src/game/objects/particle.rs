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

use crate::{
    game::objects::GameObject,
    gfx::{AnimatedTexture, Color, ColorDiff, RenderDest, RenderOptions, Renderer},
    math::Vec2,
};

/**
 * A decorative particle with a limited lifetime.
 *
 * Particles do not interact with anything (terrain or other objects.)
 */
#[derive(Clone, Debug)]
pub struct Particle {
    pos: Vec2,
    vel: Vec2,
    a: Vec2,
    reveal_in: f32,
    lifetime: Option<f32>,
    texture: AnimatedTexture,
    color: Color,
    target_color: Color,
    dcolor: ColorDiff,
}

impl Particle {
    pub fn step_mut(&mut self, timestep: f32) {
        if self.reveal_in > 0.0 {
            self.reveal_in -= timestep;
            return;
        }

        self.vel = self.vel + self.a * timestep;
        self.pos = self.pos + self.vel * timestep;

        match self.lifetime {
            Some(lt) => {
                self.lifetime = Some(lt - timestep);
                self.texture.step(timestep);
            }
            None => {
                if self.texture.step(timestep) {
                    self.lifetime = Some(0.0);
                }
            }
        }

        self.color = self.color + self.dcolor * timestep;
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        if self.reveal_in <= 0.0 {
            self.texture.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(self.pos - camera_pos),
                    color: self.color,
                    ..Default::default()
                },
            );
        }
    }
}

impl mlua::FromLua for Particle {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let color =
                Color::from_argb_u32(table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff));
            let target_color = table
                .get::<Option<u32>>("target_color")?
                .map(|c| Color::from_argb_u32(c))
                .unwrap_or(color);

            let lifetime = table.get::<Option<f32>>("lifetime")?;

            let dcolor = if let Some(l) = lifetime {
                (target_color - color) / l
            } else {
                ColorDiff::new()
            };

            Ok(Particle {
                pos: table.get("pos")?,
                vel: table.get::<Option<Vec2>>("vel")?.unwrap_or_default(),
                a: table.get::<Option<Vec2>>("a")?.unwrap_or_default(),
                lifetime,
                reveal_in: table.get::<Option<f32>>("reveal_in")?.unwrap_or(0.0),
                texture: AnimatedTexture::new(table.get("texture")?),
                color,
                target_color,
                dcolor,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Particle".to_owned(),
                message: Some("expected a table describing a particle".to_string()),
            })
        }
    }
}

impl GameObject for Particle {
    fn is_destroyed(&self) -> bool {
        match self.lifetime {
            Some(lt) => lt <= 0.0,
            None => false,
        }
    }

    fn pos(&self) -> Vec2 {
        self.pos
    }

    fn radius(&self) -> f32 {
        1.0 // todo texture width/2
    }
}
