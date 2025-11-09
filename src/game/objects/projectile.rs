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

use core::ops::Deref;
use log::error;
use mlua::{self, Table};

use crate::{
    game::{
        level::{
            Level,
            terrain::{self, Terrain},
        },
        objects::{GameObject, TerrainCollisionMode},
    },
    gfx::{AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TextureId},
    math::Vec2,
};

use super::physicalobj::PhysicalObject;

/**
 * A bullet, missile, mine, or other thing that can deal damage.
 *
 * At the world level, projectiles are divided into two categories: fast movers and slow movers.
 * Fast movers typically (but not necessarily) move fast. They do not collide with other fast movers.
 * Examples of fast movers include  basic bullets and missiles.
 *
 * Slow movers are typically (but not necessarily) stationary, and do collide with projectiles and other slow movers.
 * Examples of slow movers include mines (stationary)
 *
 * To detect friendly fire, projectiles have an owner field. Whether this field is checked
 * depends on the projectile type, but generally fast movers will not harm the ship that fired them.
 */
#[derive(Clone, Debug)]
pub struct Projectile {
    phys: PhysicalObject,
    texture: AnimatedTexture,
    color: Color,
    owner: i32,
    destroyed: bool,
    state: Option<Table>,

    /// Callback function called when the bullet hits something.
    /// function on_impact(this, terrain, ship|nil)
    on_impact: Option<mlua::Function>,

    timer: Option<f32>,
    timer_accumulator: f32,
}

impl mlua::FromLua for Projectile {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let terrain_collision_mode = {
                let mode = table.get::<Option<mlua::String>>("terrain_collision")?;
                if let Some(mode) = mode {
                    match mode.as_bytes().deref() {
                        b"exact" => TerrainCollisionMode::Exact,
                        b"simple" => TerrainCollisionMode::Simple,
                        b"passthrough" => TerrainCollisionMode::Passthrough,
                        _ => {
                            return Err(mlua::Error::FromLuaConversionError {
                                from: "string",
                                to: "TerrainCollisionMode".to_owned(),
                                message: None,
                            });
                        }
                    }
                } else {
                    TerrainCollisionMode::Exact
                }
            };

            Ok(Projectile {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get("vel")?,
                    imass: 1.0 / table.get::<Option<f32>>("mass")?.unwrap_or(30.0),
                    radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                    drag: table.get::<Option<f32>>("drag")?.unwrap_or(0.0025),
                    impulse: Vec2::ZERO,
                    terrain_collision_mode,
                },
                texture: AnimatedTexture::new(table.get("texture")?),
                owner: table.get::<Option<i32>>("owner")?.unwrap_or(0),
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                destroyed: false,
                state: table.get("state")?,
                on_impact: table.get("on_impact")?,
                timer: table.get("timer")?,
                timer_accumulator: 0.0,
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

impl mlua::UserData for Projectile {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("is_projectile", true);

        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_set("vel", |_, this, v: Vec2| {
            this.phys.vel = v;
            Ok(())
        });
        fields.add_field_method_get("owner", |_, this| Ok(this.owner));
        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));
        fields.add_field_method_set("texture", |_, this, t: TextureId| {
            this.texture = AnimatedTexture::new(t);
            Ok(())
        });
        fields.add_field_method_set("color", |_, this, c: u32| {
            this.color = Color::from_argb_u32(c);
            Ok(())
        });
        fields.add_field_method_get("timer", |_, this| Ok(this.timer));
        fields.add_field_method_set("timer", |_, this, timeout: Option<f32>| {
            this.timer = timeout;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("impulse", |_, this, v: Vec2| {
            this.phys.add_impulse(v);
            Ok(())
        });

        methods.add_method_mut("destroy", |_, this, _: ()| {
            this.destroy();
            Ok(())
        });

        methods.add_method_mut("disown", |_, this, _: ()| {
            this.owner = 0;
            Ok(())
        });
    }
}

impl Projectile {
    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn owner(&self) -> i32 {
        self.owner
    }

    pub fn destroy(&mut self) {
        self.destroyed = true;
    }

    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn impact<T: mlua::UserData + 'static>(
        &mut self,
        ter: Terrain,
        obj: Option<&mut T>,
        lua: &mlua::Lua,
    ) {
        if let Some(func) = self.on_impact.as_ref() {
            let func = func.clone();
            if let Err(err) = lua.scope(|scope| {
                func.call::<()>((
                    scope.create_userdata_ref_mut(self)?,
                    ter,
                    match obj {
                        Some(s) => mlua::Value::UserData(scope.create_userdata_ref_mut(s)?),
                        None => mlua::Value::Nil,
                    },
                ))
            }) {
                error!("Projectile impact script error: {}", err);
            }
        }
    }

    pub fn step_mut(&mut self, level: &Level, lua: &mlua::Lua, timestep: f32) {
        let (_, ter) = self.phys.step(level, timestep);

        if terrain::is_solid(ter) {
            self.impact::<Self>(ter, None, lua);
        }

        self.texture.step(timestep);

        if let Some(timer) = self.timer.as_mut() {
            *timer -= timestep;
            self.timer_accumulator += timestep;
            let acc = self.timer_accumulator;

            if *timer <= 0.0 {
                self.timer_accumulator = 0.0;
                match lua.scope(|scope| {
                    lua.globals()
                        .get::<mlua::Function>("luola_on_object_timer")?
                        .call::<Option<f32>>((scope.create_userdata_ref_mut(self)?, acc))
                }) {
                    Ok(rerun) => {
                        self.timer = rerun;
                    }
                    Err(err) => {
                        error!("Projectile timer: {err}");
                        self.timer = None;
                    }
                };
            }
        }
    }

    pub fn step(&self, level: &Level, lua: &mlua::Lua, timestep: f32) -> Projectile {
        let mut p = self.clone();
        p.step_mut(level, lua, timestep);
        p
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        self.texture.render(
            renderer,
            &RenderOptions {
                dest: RenderDest::Centered(self.phys.pos - camera_pos),
                color: self.color,
                mode: if self.texture.id().needs_rotation() {
                    RenderMode::Rotated(self.phys.vel.angle(), false)
                } else {
                    RenderMode::Normal
                },
                ..Default::default()
            },
        );
    }
}

impl GameObject for Projectile {
    fn pos(&self) -> Vec2 {
        self.phys.pos
    }

    fn radius(&self) -> f32 {
        self.phys.radius
    }

    fn is_destroyed(&self) -> bool {
        self.destroyed
    }
}
