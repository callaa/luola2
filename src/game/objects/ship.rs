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

use log::error;
use mlua::{Function, Lua, Table, UserData};

use super::{GameObject, PhysicalObject, SCALE_FACTOR, TerrainCollisionMode};
use crate::game::controller::GameController;
use crate::game::level::{Level, terrain};
use crate::gfx::{Color, RenderDest, RenderMode, RenderOptions, Renderer, TexAlt, TextureId};
use crate::math::Vec2;

/**
 * A ship that can be piloted by a player.
 *
 * A ship can be abandonded, in which case its player id and controller properties set to zero.
 * Abandoned ships can be claimed by pilots.
 *
 * A ship's hitpoints may go negative: in this case the ship is in a wrecked state, falling
 * uncontrollably. (Future: pilot may eject in this state)
 *
 *
 * A non-local or AI controlled ship will have a player ID but no controller.
 */
#[derive(Clone, Debug)]
pub struct Ship {
    phys: PhysicalObject,

    /// The angle the ship is facing
    angle: f32,

    /// Engine thrust force
    thrust: f32,

    /// Maximum turning speed in degrees per second
    turn_speed: f32,

    /// The controller used to control this ship.
    /// Zero or negative if no controller is attached
    controller: i32,

    /// The ID of the player occupying this ship.
    /// Zero or negative if ship is unoccupied.
    player_id: i32,

    /// Remaining health
    /// When health reaches zero (or below,) the ship is wrecked and longer operable.
    /// The on_wrecked callback is called at this point.
    /// When a dead ship hits terrain (or HP reaches a sufficiently large negative value,)
    /// the on_destroyed callback will be called.
    hitpoints: f32,

    /// Maximum health
    max_hitpoints: f32,

    /// Number of seconds the primary weapon is still on cooldown
    primary_weapon_cooldown: f32,

    /// Number of seconds the secondary weapon is still on cooldown
    secondary_weapon_cooldown: f32,

    /// Number of ammo units remaining
    ammo_remaining: f32,

    /// Maximum ammo
    max_ammo: f32,

    /// Timer for damage effect
    damage_effect: f32,

    /**
     * Function to call when firing the primary weapon
     *
     * The function takes a mutable reference to the ship as a parameter.
     * It should set the primary weapon's cooldown.
     */
    on_fire_primary: Option<Function>,

    /**
     * Function to call when firing the secondary weapon
     *
     * The function takes a mutable reference to the ship as a parameter.
     * It should set the secondary weapon's cooldown and remaining ammo.
     * The function is called regardless of how much ammo remains.
     */
    on_fire_secondary: Option<Function>,

    /**
     * Function to call on every tick while ship is thrusting
     *
     * This is typically used to play a sound effect and create exhaust particle
     * effects.
     */
    on_thrust: Option<Function>,

    /**
     * Function to call when the ship is destroyed.
     */
    on_destroyed: Option<Function>,

    /**
     * Function to call while ship is sitting on a base
     *
     * Signature: function(ship, timestep, is_underwater)
     */
    on_base: Option<Function>,

    /// Ship object ready to be deleted
    /// Ship destruction typically triggers an end-of-level condition check
    destroyed: bool,

    /// Used to select sprite
    engine_active: bool,

    /// Cloaking device active: use special rendering mode
    cloaked: bool,

    /// Ghost mode active: terrain collisions disabled and special rendering mode used
    ghostmode: bool,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,

    /// Extra state for scripting
    state: Table,

    /// Texture to draw the ship with
    texture: TextureId,
}

impl UserData for Ship {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("texture", |_, this| Ok(this.texture));
        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_get("radius", |_, this| Ok(this.phys.radius));
        fields.add_field_method_get("angle", |_, this| Ok(this.angle));
        fields.add_field_method_get("player", |_, this| Ok(this.player_id));
        fields.add_field_method_get("health", |_, this| Ok(this.hitpoints));
        fields.add_field_method_get("ammo", |_, this| Ok(this.ammo_remaining));
        fields.add_field_method_set("ammo", |_, this, ammo: f32| {
            this.ammo_remaining = ammo.clamp(0.0, this.max_ammo);
            Ok(())
        });
        fields.add_field_method_get("cloaked", |_, this| Ok(this.cloaked));
        fields.add_field_method_set("cloaked", |_, this, c: bool| {
            this.cloaked = c;
            Ok(())
        });
        fields.add_field_method_get("ghostmode", |_, this| Ok(this.ghostmode));
        fields.add_field_method_set("ghostmode", |_, this, gm: bool| {
            this.set_ghostmode(gm);
            Ok(())
        });
        fields.add_field_method_get("timer", |_, this| Ok(this.timer));
        fields.add_field_method_set("timer", |_, this, timeout: Option<f32>| {
            this.timer = timeout;
            Ok(())
        });
        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));
        fields.add_field_method_set("primary_weapon_cooldown", |_, this, cooldown| {
            this.primary_weapon_cooldown = cooldown;
            Ok(())
        });
        fields.add_field_method_set("secondary_weapon_cooldown", |_, this, cooldown| {
            this.secondary_weapon_cooldown = cooldown;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        // apply damage to the ship with side effects
        methods.add_method_mut("damage", |_, this, hp: f32| {
            this.damage(hp);
            Ok(())
        });
        // Consume ammo if there is enough and set secondary weapon cooldown..
        methods.add_method_mut("consume_ammo", |_, this, (amount, cooldown): (f32, f32)| {
            let a = this.ammo_remaining - amount;
            Ok(if a < 0.0 {
                false
            } else {
                this.ammo_remaining = a;
                this.secondary_weapon_cooldown = cooldown;
                true
            })
        });
    }
}

impl mlua::FromLua for Ship {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let hitpoints = table.get::<Option<f32>>("hitpoints")?.unwrap_or(100.0);
            let ammo = table.get::<Option<f32>>("ammo")?.unwrap_or(100.0);
            Ok(Ship {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get::<Option<Vec2>>("vel")?.unwrap_or_default(),
                    imass: 1.0 / table.get::<Option<f32>>("mass")?.unwrap_or(1000.0),
                    radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                    drag: table.get::<Option<f32>>("drag")?.unwrap_or(0.025),
                    impulse: Vec2::ZERO,
                    terrain_collision_mode: TerrainCollisionMode::Simple,
                },
                angle: table.get::<Option<f32>>("angle")?.unwrap_or(0.0),
                thrust: table.get::<Option<f32>>("thrust")?.unwrap_or(50.0),
                turn_speed: table.get::<Option<f32>>("turn_speed")?.unwrap_or(260.0),
                player_id: table.get::<Option<i32>>("player")?.unwrap_or(0),
                controller: table.get::<Option<i32>>("controller")?.unwrap_or(0),
                hitpoints,
                max_hitpoints: hitpoints,
                primary_weapon_cooldown: 0.0,
                on_fire_primary: table.get("on_fire_primary")?,
                secondary_weapon_cooldown: 0.0,
                on_fire_secondary: table.get("on_fire_secondary")?,
                on_thrust: table.get("on_thrust")?,
                on_destroyed: table.get("on_destroyed")?,
                on_base: table.get("on_base")?,
                ammo_remaining: ammo,
                max_ammo: ammo,
                damage_effect: 0.0,
                state: table
                    .get::<Option<Table>>("state")?
                    .unwrap_or_else(|| lua.create_table().unwrap()),
                texture: table.get("texture")?,
                destroyed: false,
                engine_active: false,
                cloaked: false,
                ghostmode: false,
                timer: table.get("timer")?,
                timer_accumulator: 0.0,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Ship".to_owned(),
                message: Some("expected a table describing a ship".to_string()),
            })
        }
    }
}

impl Ship {
    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn physics_mut(&mut self) -> &mut PhysicalObject {
        &mut self.phys
    }

    pub fn player_id(&self) -> i32 {
        self.player_id
    }

    pub fn controller(&self) -> i32 {
        self.controller
    }

    pub fn health(&self) -> f32 {
        self.hitpoints.max(0.0) / self.max_hitpoints
    }

    pub fn ammo(&self) -> f32 {
        self.ammo_remaining / self.max_ammo
    }

    pub fn is_wrecked(&self) -> bool {
        self.hitpoints <= 0.0
    }

    /**
     * Inflict damage on this ship.
     *
     * Damage can be negative, in which case it will repair the ship.
     * Hitpoints can go negative (wreckage state,) but they cannot go above the maximum
     */
    pub fn damage(&mut self, hp: f32) {
        //let was_wrecked = self.is_wrecked();
        self.hitpoints = (self.hitpoints - hp).min(self.max_hitpoints);

        if hp > 0.0 {
            self.damage_effect = 0.1;
        }

        /* TODO on_wrecked callback
        if self.hitpoints <= 0 && !was_wrecked {
        }
        */
    }

    pub fn destroy(&mut self, lua: &Lua) {
        if !self.destroyed {
            self.destroyed = true;
            if let Some(func) = self.on_destroyed.as_ref() {
                let func = func.clone();
                if let Err(err) =
                    lua.scope(|scope| func.call::<()>(scope.create_userdata_ref_mut(self).unwrap()))
                {
                    error!("Ship on_destroyed callback: {err}");
                }
            }
        }
    }

    fn set_ghostmode(&mut self, gm: bool) {
        self.ghostmode = gm;
        if gm {
            self.phys.terrain_collision_mode = TerrainCollisionMode::None;
        } else {
            self.phys.terrain_collision_mode = TerrainCollisionMode::Simple;
        }
    }

    /**
     * Perform a simulation step and return a new copy of the ship
     */
    pub fn step(
        &self,
        controller: Option<&GameController>,
        level: &Level,
        lua: &Lua,
        timestep: f32,
    ) -> Ship {
        let mut ship = self.clone();

        if !self.is_wrecked()
            && let Some(controller) = controller
        {
            ship.angle += ship.turn_speed * controller.turn * timestep;

            if controller.thrust {
                ship.phys.vel = ship.phys.vel
                    + Vec2::for_angle(-ship.angle, SCALE_FACTOR * self.thrust * timestep);
                ship.engine_active = true;
            } else {
                ship.engine_active = false;
            }
        }

        let impact_speed_squared = ship.phys.vel.magnitude_squared();

        let (prev_ter, ter) = ship.phys.step(level, timestep);

        if ship.engine_active
            && let Some(func) = self.on_thrust.as_ref()
        {
            if let Err(err) = lua.scope(|scope| {
                func.call::<()>((
                    scope.create_userdata_ref_mut(&mut ship).unwrap(),
                    terrain::is_underwater(ter),
                ))
            }) {
                error!("Ship on_thrust callback: {err}");
            }
        }

        if ship.damage_effect > 0.0 {
            ship.damage_effect -= timestep;
        }

        if terrain::is_underwater(prev_ter) != terrain::is_underwater(ter) {
            // Water/air transition
            match lua.globals().get::<Function>("luola_splash") {
                Ok(func) => {
                    if let Err(err) = func.call::<()>((ship.pos(), ship.phys.vel, ship.phys.imass))
                    {
                        error!("luola_spash error: {err}");
                    }
                }
                Err(err) => {
                    error!("Couldn't get splash function: {err}");
                }
            }
        }

        if terrain::is_base(ter) {
            // Bases forcibly orient the ship
            if terrain::is_underwater(ter) {
                ship.angle = 270.0;
            } else {
                ship.angle = 90.0;
            }

            // Repair/resupply logic is implemented in scripts to allow
            // for differences between ship types and so we can do special effects.
            if let Some(func) = self.on_base.as_ref() {
                if let Err(err) = lua.scope(|scope| {
                    func.call::<()>((
                        scope.create_userdata_ref_mut(&mut ship).unwrap(),
                        timestep,
                        terrain::is_underwater(ter),
                    ))
                }) {
                    error!("Ship on_base callback: {err}");
                }
            }
        }

        if terrain::is_solid(ter) && impact_speed_squared > 100000.0 {
            // TODO scale damage based on speed?
            ship.damage(1.0);
        }

        if terrain::is_indestructible_solid(ter)
            && terrain::is_indestructible_solid(level.terrain_at(ship.pos()))
        {
            // if a ship gets stuck inside indestructable terrain, it can soft-lock the round
            // (we also don't want to give ghostmode users safe camping areas)
            ship.damage(1.0);
        }

        if ship.is_wrecked() {
            ship.hitpoints -= timestep * 100.0; // wreckage deteriorates
            ship.angle += 19.0; // spinning effect

            if ship.hitpoints < -1000.0 || terrain::is_solid(ter) {
                ship.destroy(lua);
            }
        } else if let Some(controller) = controller {
            if ship.primary_weapon_cooldown > 0.0 {
                ship.primary_weapon_cooldown -= timestep;
            }

            if ship.secondary_weapon_cooldown > 0.0 {
                ship.secondary_weapon_cooldown -= timestep;
            }

            if controller.fire_primary
                && ship.primary_weapon_cooldown <= 0.0
                && let Some(func) = self.on_fire_primary.as_ref()
            {
                if let Err(err) = lua.scope(|scope| {
                    func.call::<()>(scope.create_userdata_ref_mut(&mut ship).unwrap())
                }) {
                    error!("Ship on_primary_fire callback: {err}");
                }
            }

            if controller.fire_secondary
                && ship.secondary_weapon_cooldown <= 0.0
                && let Some(func) = self.on_fire_secondary.as_ref()
            {
                if let Err(err) = lua.scope(|scope| {
                    func.call::<()>(scope.create_userdata_ref_mut(&mut ship).unwrap())
                }) {
                    error!("Ship on_secondary_fire callback: {err}",);
                }
            }
        }

        if let Some(timer) = ship.timer.as_mut() {
            *timer -= timestep;
            ship.timer_accumulator += timestep;
            let acc = ship.timer_accumulator;

            if *timer <= 0.0 {
                ship.timer_accumulator = 0.0;
                match lua.scope(|scope| {
                    lua.globals()
                        .get::<mlua::Function>("luola_on_object_timer")?
                        .call::<Option<f32>>((scope.create_userdata_ref_mut(&mut ship)?, acc))
                }) {
                    Ok(rerun) => {
                        ship.timer = rerun;
                    }
                    Err(err) => {
                        error!("Ship timer : {err}");
                        ship.timer = None;
                    }
                };
            }
        }

        ship
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        let ts = renderer.texture_store();

        let tex = ts.get_texture_alt_fallback(
            self.texture,
            if self.engine_active {
                TexAlt::Active
            } else {
                TexAlt::Main
            },
        );

        let mut renderopts = RenderOptions {
            dest: RenderDest::Centered(self.phys.pos - camera_pos),
            mode: RenderMode::Rotated(self.angle, false),
            ..Default::default()
        };

        if self.ghostmode {
            renderopts.color = Color::new_rgba(1.0, 1.0, 1.0, 0.3);
            renderopts.dest = RenderDest::Centered(
                self.phys.pos - camera_pos
                    + Vec2(fastrand::f32() * 8.0 - 4.0, fastrand::f32() * 8.0 - 4.0),
            );
        }

        let cloaked = self.cloaked && self.damage_effect <= 0.0 && !self.is_wrecked();
        if cloaked {
            renderopts.color = Color::new_rgba(0.0, 0.0, 0.0, 0.5);
        }

        tex.render(renderer, &renderopts);

        if self.damage_effect > 0.0
            && let Some(damage) = ts.get_texture_alt(self.texture, TexAlt::Damage)
        {
            damage.render(renderer, &renderopts);
        } else if self.player_id > 0
            && !cloaked
            && let Some(decal) = ts.get_texture_alt(self.texture, crate::gfx::TexAlt::Decal)
        {
            renderopts.color = Color::player_color(self.player_id);
            if self.ghostmode {
                renderopts.color = renderopts.color.with_alpha(0.3);
            }
            decal.render(renderer, &renderopts);
        }
    }
}

impl GameObject for Ship {
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
