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
use std::fs::read_to_string;
use std::{cell::RefCell, path::Path, rc::Rc};

use anyhow::{Result, anyhow};
use log::error;
use mlua::{
    Either, FromLua, Function, Lua, Result as LuaResult, String as LuaString, Table, Value,
};

use crate::fs::find_datafile_path;
use crate::game::level::Level;
use crate::game::objects::{Critter, GameObject, GameObjectArray, Particle, Projectile, Ship};
use crate::game::world::WorldEffect;
use crate::gfx::Renderer;
use crate::math::{LineF, RectF, Vec2};

pub struct ScriptEnvironment {
    lua: Lua,
    effect_accumulator: Rc<RefCell<Vec<WorldEffect>>>,
    global_timer: Rc<RefCell<Option<f32>>>,
    global_timer_accumulator: f32,
}

impl ScriptEnvironment {
    pub fn create_lua(renderer: Rc<RefCell<Renderer>>) -> Result<Lua> {
        let lua = Lua::new();

        let script_path = find_datafile_path(&["script"])?;

        // Load modules from the script path only
        lua.globals()
            .get::<Table>("package")?
            .set("path", format!("{}/?.lua", script_path.to_str().unwrap()))?;

        let texapi = lua.create_table()?;
        texapi.set(
            "get",
            lua.create_function(move |_, name: String| {
                Ok(renderer.borrow().texture_store().find_texture(&name)?)
            })?,
        )?;

        lua.globals().set("textures", texapi)?;
        Ok(lua)
    }

    pub fn new(renderer: Rc<RefCell<Renderer>>) -> Result<Self> {
        let lua = Self::create_lua(renderer)?;

        let effect_accumulator = Rc::new(RefCell::new(Vec::new()));

        Ok(Self {
            lua,
            effect_accumulator,
            global_timer: Rc::new(RefCell::new(None)),
            global_timer_accumulator: 0.0,
        })
    }

    pub fn load_level_specific_script(&mut self, path: &Path) -> LuaResult<()> {
        let script_content = read_to_string(path)?;

        // Add script root to search path so we can require additional level specific modules
        let package = self.lua.globals().get::<Table>("package")?;

        let old_searchpath = package.get::<String>("path")?;
        let script_path = path.parent().expect("script path should have a parent");
        package.set(
            "path",
            format!("{};{}/?.lua", old_searchpath, script_path.to_str().unwrap()),
        )?;

        // Load new script
        self.lua.load(script_content).exec()?;
        Ok(())
    }

    pub fn init_game(
        &mut self,
        level: Rc<RefCell<Level>>,
        ship_list: Rc<RefCell<GameObjectArray<Ship>>>,
        critter_list: Rc<RefCell<GameObjectArray<Critter>>>,
    ) -> LuaResult<()> {
        let api = self.lua.create_table().unwrap();

        // Find a spawnpoint for a ship
        {
            let level = level.clone();
            api.set(
                "find_spawnpoint",
                self.lua.create_function(
                    move |_, (rect, allow_water): (Option<RectF>, Option<bool>)| {
                        Ok(level
                            .borrow()
                            .find_spawnpoint(rect, allow_water.unwrap_or(false))?)
                    },
                )?,
            )?;
        }

        // Check terrain type
        // function terrain_at(pos) -> Terrain
        {
            let level = level.clone();
            api.set(
                "terrain_at",
                self.lua
                    .create_function(move |_, pos: Vec2| Ok(level.borrow().terrain_at(pos)))?,
            )?;
        }

        // Terrain line intersection check
        // function terrain_line(start, end) -> (Vec2, Terrain, bool), where bool is true if intersected with solid terrain
        {
            let level = level.clone();
            api.set(
                "terrain_line",
                self.lua
                    .create_function(move |_, (start, end): (Vec2, Vec2)| {
                        match level.borrow().terrain_line(LineF(start, end)) {
                            Either::Left((t, pos)) => Ok((pos, t, true)),
                            Either::Right(t) => Ok((end, t, false)),
                        }
                    })?,
            )?;
        }

        // Wrap TextureStore::find_texture

        // Iterate through a read-only list of ships
        // function ships_iter(callback)
        // Callback can return false to stop iteration
        api.set(
            "ships_iter",
            self.lua.create_function(move |lua, callback: Function| {
                let ships = ship_list.borrow();
                lua.scope(|scope| {
                    for ship in ships.iter() {
                        let res = callback.call::<Option<bool>>(scope.create_userdata_ref(ship))?;
                        if let Some(false) = res {
                            break;
                        }
                    }
                    Ok(())
                })
            })?,
        )?;

        // Iterate through a read-only list of critters within the given position and radius
        // function critters_iter(callback, pos, except_this_id, radius)
        api.set(
            "critters_iter",
            self.lua.create_function(
                move |lua, (pos, rad, except_this, callback): (Vec2, f32, u32, Function)| {
                    let critters = critter_list.borrow();
                    let rr = rad * rad;
                    lua.scope(|scope| {
                        for critter in critters.range_slice(pos.0 - rad, pos.0 + rad) {
                            if critter.id() != except_this && critter.pos().dist_squared(pos) < rr {
                                let res = callback
                                    .call::<Option<bool>>(scope.create_userdata_ref(critter))?;
                                if let Some(false) = res {
                                    break;
                                }
                            }
                        }
                        Ok(())
                    })
                },
            )?,
        )?;

        // Change the world.
        // Effects will be applied at the very end of the animation step.
        let efacc = self.effect_accumulator.clone();
        api.set(
            "effect",
            self.lua
                .create_function(move |lua, (effect_type, props): (LuaString, Value)| {
                    let effect = match effect_type.as_bytes().deref() {
                        b"AddBullet" => WorldEffect::AddBullet(Projectile::from_lua(props, lua)?),
                        b"AddMine" => WorldEffect::AddMine(Projectile::from_lua(props, lua)?),
                        b"MakeBulletHole" => {
                            WorldEffect::MakeBulletHole(Vec2::from_lua(props, lua)?)
                        }
                        b"MakeBigHole" => {
                            if let mlua::Value::Table(t) = props {
                                let pos = t.get("pos")?;
                                let r: i32 = t.get("r")?;
                                WorldEffect::MakeBigHole(pos, r.clamp(1, 999))
                            } else {
                                return Err(anyhow!("expected {{pos, r}}").into());
                            }
                        }
                        b"AddParticle" => WorldEffect::AddParticle(Particle::from_lua(props, lua)?),
                        b"AddShip" => WorldEffect::AddShip(Ship::from_lua(props, lua)?),
                        b"AddCritter" => WorldEffect::AddCritter(Critter::from_lua(props, lua)?),
                        b"EndRound" => WorldEffect::EndRound(i32::from_lua(props, lua)?),
                        unknown => return Err(anyhow!("Unknown effect type: {:?}", unknown).into()),
                    };

                    efacc.borrow_mut().push(effect);

                    Ok(())
                })?,
        )?;

        // Global timer
        // When timer has a value and reaches zero, the function "luola_global_on_timer" is executed
        let global_timer = self.global_timer.clone();
        api.set(
            "set_global_timer",
            self.lua.create_function(move |_, timeout: f32| {
                *global_timer.borrow_mut() = Some(timeout);
                Ok(())
            })?,
        )?;

        // Access to game objects via the "game" table
        let globals = self.lua.globals();
        globals.set("game", api)?;

        // Constructors for common types
        globals.set(
            "Vec2",
            self.lua
                .create_function(|_, (x, y): (f32, f32)| Ok(Vec2(x, y)))?,
        )?;

        globals.set(
            "Vec2_for_angle",
            self.lua
                .create_function(|_, (a, m): (f32, f32)| Ok(Vec2::for_angle(a, m)))?,
        )?;

        globals.set(
            "RectF",
            self.lua
                .create_function(|_, (x, y, w, h): (f32, f32, f32, f32)| {
                    Ok(RectF::new(x, y, w, h))
                })?,
        )?;

        // Load main entrypoint file
        self.lua.load(r#"require "luola_main""#).exec()?;

        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    pub fn get_function(&self, name: &str) -> LuaResult<Function> {
        self.lua.globals().get::<Function>(name)
    }

    pub fn take_accumulated_effects(&mut self) -> Vec<WorldEffect> {
        self.effect_accumulator.take()
    }

    pub fn step_global_timer(&mut self, timestep: f32) {
        let mut global_timer = self.global_timer.borrow_mut();

        if let Some(gt) = global_timer.as_mut() {
            // the accumulator preserves the actual time elapsed since last
            // timer invocation even if the timer is reset in between.
            self.global_timer_accumulator += timestep;
            *gt -= timestep;

            if *gt <= 0.0 {
                if let Ok(callback) = self.get_function("luola_on_global_timer") {
                    drop(global_timer);

                    match callback.call::<Option<f32>>(self.global_timer_accumulator) {
                        Ok(rerun) => {
                            self.global_timer.replace(rerun);
                        }
                        Err(err) => {
                            error!("luola_global_on_timer callback execution failed: {err}");
                            self.global_timer.replace(None);
                        }
                    }
                }
                self.global_timer_accumulator = 0.0;
            }
        }
    }
}
