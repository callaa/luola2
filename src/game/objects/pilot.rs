use anyhow::anyhow;

use crate::{
    call_state_method,
    game::{
        GameController, PlayerId,
        level::{LEVEL_SCALE, Level, TerrainLineHit, terrain},
        objects::{GameObject, PhysicalObject, Rope, Ship, TerrainCollisionMode},
    },
    gameobject_timer, get_state_method,
    gfx::{AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TexAlt},
    math::{LineF, Vec2},
};

#[derive(Clone)]
pub struct Pilot {
    phys: PhysicalObject,
    player_id: PlayerId,
    controller: i32,
    destroyed: bool,
    jetpack_charge: f32,
    state: Option<mlua::Table>,
    stand_texture: AnimatedTexture,
    jetpack_texture: AnimatedTexture,
    walk_texture: AnimatedTexture,
    swim_texture: AnimatedTexture,
    parachute_texture: AnimatedTexture,
    mode: MotionMode,
    auto_target: Option<Vec2>,
    aim_mode: bool,
    aim_angle: f32, // -90 -- 90
    facing: i8,     // -1 or 1
    ninjarope: NinjaRope,
    /// Is the fire3 button being held down? Used to detect leading-edge input event
    /// for ninjarope activation.
    fire3_down: bool,
    weapon_cooldown: f32,
    timer: Option<f32>,
    timer_accumulator: f32,
}

#[derive(Clone)]
enum NinjaRope {
    Stowed,
    Extending { vec: Vec2, length: f32 },
    Attached(Rope),
}

#[derive(Clone, Debug)]
enum MotionMode {
    Standing,
    Walking,
    Parachuting,
    Jetpacking,
    Swimming,
    Ninjaroping,
}

impl mlua::UserData for Pilot {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("is_pilot", true);
        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_get("facing", |_, this| Ok(this.facing));
        fields.add_field_method_get("player", |_, this| Ok(this.player_id));
        fields.add_field_method_get("controller", |_, this| Ok(this.controller));
        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));

        fields.add_field_method_set("weapon_cooldown", |_, this, cooldown| {
            this.weapon_cooldown = cooldown;
            Ok(())
        });

        fields.add_field_method_get("rope_tangent", |_, this| {
            if let NinjaRope::Attached(rope) = &this.ninjarope {
                let rv = (rope.endpoint() - this.phys.pos).normalized();
                Ok(Vec2(-rv.1, rv.0))
            } else {
                Err(anyhow!("Ninjarope not attached!").into())
            }
        });
        fields.add_field_method_get("timer", |_, this| Ok(this.timer));
        fields.add_field_method_set("timer", |_, this, timeout: Option<f32>| {
            this.timer = timeout;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("aim_vector", |_, this, mag: f32| {
            Ok(this.aim_vector(mag, false))
        });
        methods.add_method_mut("destroy", |_, this, _: ()| {
            this.destroy();
            Ok(())
        });
        methods.add_method_mut("impulse", |_, this, v: Vec2| {
            this.phys.add_impulse(v);
            Ok(())
        });
    }
}

const NORMAL_DRAG: f32 = 0.015;
const PARACHUTE_DRAG: f32 = 0.7;
const DANGER_SPEED: f32 = 500.0;
const NINJAROPE_SPEED: f32 = 800.0;
const NINJAROPE_MAX_LEN: f32 = 300.0;
const NINJAROPE_CLIMB_SPEED: f32 = 200.0;

impl mlua::FromLua for Pilot {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            Ok(Pilot {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get::<Option<Vec2>>("vel")?.unwrap_or_default(),
                    imass: 1.0 / 300.0,
                    radius: table.get::<Option<f32>>("radius")?.unwrap_or(6.0),
                    drag: PARACHUTE_DRAG,
                    impulse: Vec2::ZERO,
                    terrain_collision_mode: TerrainCollisionMode::Simple,
                },
                player_id: table.get("player")?,
                controller: table.get("controller")?,
                destroyed: false,
                jetpack_charge: 1.0,
                state: table.get("state")?,
                stand_texture: AnimatedTexture::new(table.get("stand_texture")?),
                jetpack_texture: AnimatedTexture::new(table.get("jetpack_texture")?),
                walk_texture: AnimatedTexture::new(table.get("walk_texture")?),
                swim_texture: AnimatedTexture::new(table.get("swim_texture")?),
                parachute_texture: AnimatedTexture::new(table.get("parachute_texture")?),
                mode: MotionMode::Parachuting,
                auto_target: None,
                aim_mode: false,
                aim_angle: 0.0,
                facing: 1,
                ninjarope: NinjaRope::Stowed,
                fire3_down: false,
                weapon_cooldown: 0.0,
                timer: table.get("timer")?,
                timer_accumulator: 0.0,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Pilot".to_owned(),
                message: Some("expected a table describing a pilot".to_string()),
            })
        }
    }
}

impl Pilot {
    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn controller(&self) -> i32 {
        self.controller
    }

    pub fn destroy(&mut self) {
        self.destroyed = true;
    }

    pub fn jetpack_charge(&self) -> f32 {
        self.jetpack_charge
    }

    const GUN_OFFSET: Vec2 = Vec2(0.0, -16.0);

    pub fn aim_vector(&self, mag: f32, force_manual: bool) -> Vec2 {
        if !(self.aim_mode || force_manual)
            && let Some(at) = self.auto_target
        {
            (at - self.pos() - Self::GUN_OFFSET).normalized() * mag
        } else {
            Vec2::for_angle(self.aim_angle, mag).element_wise_product(Vec2(self.facing as f32, 1.0))
        }
    }

    /// Get the position (in world coordinates) for the targeting reticle
    pub fn aim_target(&self) -> Option<Vec2> {
        if self.aim_mode {
            Some(self.phys.pos + Self::GUN_OFFSET + self.aim_vector(60.0, false))
        } else {
            self.auto_target
        }
    }

    pub fn set_autotarget(&mut self, target: Option<Vec2>) {
        self.auto_target = target;
    }

    // Take a step to the right or left
    fn walk(pos: Vec2, level: &Level, dir: f32) -> Option<Vec2> {
        let new_x = pos.0 + (dir * LEVEL_SCALE);

        const MAX_SLOPE: i32 = 5;

        // Downhill
        for i in 0..MAX_SLOPE {
            let new_pos = Vec2(new_x, pos.1 + i as f32 * LEVEL_SCALE);
            let ter_at = level.terrain_at(new_pos);
            if terrain::is_solid(ter_at) && !terrain::can_walk_through(ter_at) {
                // pixel above must be free
                let above = new_pos - Vec2(0.0, LEVEL_SCALE);
                if terrain::can_walk_through(level.terrain_at(above)) {
                    return Some(above);
                } else {
                    break;
                }
            }
        }

        // Uphill
        for i in 1..MAX_SLOPE {
            let new_pos = Vec2(new_x, pos.1 - i as f32 * LEVEL_SCALE);
            let ter_at = level.terrain_at(new_pos);
            if terrain::is_solid(ter_at) && !terrain::can_walk_through(ter_at) {
                // pixel above must be free
                let above = new_pos - Vec2(0.0, LEVEL_SCALE);

                if terrain::can_walk_through(level.terrain_at(above)) {
                    return Some(above);
                }
            }
        }

        // Walk off an edge
        let new_pos = Vec2(new_x, pos.1);
        if terrain::can_walk_through(level.terrain_at(new_pos)) {
            Some(new_pos)
        } else {
            None
        }
    }

    pub fn step_mut(
        &mut self,
        controller: Option<&GameController>,
        level: &Level,
        lua: &mlua::Lua,
        timestep: f32,
    ) {
        let (_old_ter, ter) = self.phys.step(level, timestep);

        if let NinjaRope::Attached(rope) = &self.ninjarope {
            debug_assert!(matches!(self.mode, MotionMode::Ninjaroping));
            rope.physics_step(&mut self.phys);
        }

        self.walk_texture.step(timestep);
        self.swim_texture.step(timestep);

        if self.phys.vel.1 > DANGER_SPEED {
            // TODO call script
            println!("Falling dangerously fast: {}", self.phys.vel);
        }

        // Hitting the ground ends parachute mode
        if matches!(self.mode, MotionMode::Parachuting) && !terrain::is_space(ter) {
            self.mode = MotionMode::Walking;
            self.phys.drag = NORMAL_DRAG;
        }

        if let Some(ctrl) = controller {
            self.aim_mode = ctrl.fire2 || ctrl.aim != 0.0;
            self.weapon_cooldown -= timestep;

            let ter_above = level.terrain_at(self.phys.pos - Vec2(0.0, LEVEL_SCALE));

            // gamepad specific aiming shortcut
            if ctrl.aim != 0.0 {
                self.adjust_aim(ctrl.aim * timestep);
            }

            if matches!(self.ninjarope, NinjaRope::Attached(_)) && ctrl.walk != 0.0 {
                // Ninjarope swing.
                // In a separate if block because we can't borrow self.ninjarope and
                // use call_state_method at the same time
                if ctrl.walk.abs() != 0.0 {
                    self.facing = -ctrl.walk.signum() as i8;
                    call_state_method!(*self, lua, "on_ninjarope_swing", ctrl.walk);
                }
            }

            if let NinjaRope::Attached(rope) = &mut self.ninjarope {
                // Check that the ninjarope attachment point still exist
                if !terrain::is_solid(level.terrain_at(rope.endpoint())) {
                    self.ninjarope = NinjaRope::Stowed;
                    self.mode = MotionMode::Jetpacking;
                } else {
                    // Climbing or aiming
                    if ctrl.thrust > 0.5 {
                        if ctrl.fire2 {
                            self.adjust_aim(timestep);
                        } else {
                            rope.adjust(-NINJAROPE_CLIMB_SPEED * timestep);
                        }
                    } else if ctrl.thrust < -0.5 && rope.length() < NINJAROPE_MAX_LEN {
                        if ctrl.fire2 {
                            self.adjust_aim(-timestep);
                        } else {
                            rope.adjust(NINJAROPE_CLIMB_SPEED * timestep);
                        }
                    }
                }
            } else {
                // Regular movement controls
                if ctrl.walk.abs() > 0.1 {
                    // Horizontal motion
                    let dir = -ctrl.walk.signum();
                    self.facing = dir as i8;

                    if matches!(self.mode, MotionMode::Parachuting) {
                        // Drifting
                        self.phys.add_impulse(Vec2(dir * 2000.0, 1000.0));
                    } else if terrain::is_solid(ter)
                        && let Some(newpos) = Self::walk(self.pos(), level, dir)
                    {
                        // Walking on solid ground
                        self.phys.pos = newpos;
                        self.mode = MotionMode::Walking;
                    } else if terrain::is_water(ter) {
                        // Swimming underwater
                        self.phys.add_impulse(Vec2(dir * 1000.0, 0.0));
                        self.mode = MotionMode::Swimming;
                    }
                } else if terrain::is_solid(ter) && terrain::is_space(ter_above) {
                    self.mode = MotionMode::Standing;
                } else if terrain::is_underwater(ter) {
                    self.mode = MotionMode::Swimming;
                } else if !matches!(self.mode, MotionMode::Parachuting | MotionMode::Jetpacking) {
                    self.mode = MotionMode::Standing
                }

                if ctrl.thrust > 0.5 {
                    if ctrl.fire2 {
                        // Aim upwards
                        self.adjust_aim(timestep);
                    } else if terrain::is_underwater(ter) {
                        // Swim up
                        self.phys.add_impulse(Vec2(0.0, -2000.0));
                    }
                }

                if ctrl.thrust < 0.0 {
                    if ctrl.fire2 || ctrl.aim < 0.0 {
                        // Aim downwards
                        self.adjust_aim(-timestep);
                    } else if matches!(self.mode, MotionMode::Parachuting) {
                        // Descend faster
                        self.phys.add_impulse(Vec2(0.0, 1000.0));
                    } else if terrain::is_water(ter) {
                        // Swim down
                        self.phys.add_impulse(Vec2(0.0, 2000.0));
                    }
                }

                if ctrl.eject && self.weapon_cooldown <= 0.0 {
                    // Ship recall
                    // Note: this needs a cooldown too so we reuse the weapon cooldown.
                    call_state_method!(*self, lua, "on_ship_recall", ter);
                    self.weapon_cooldown = 0.5;
                }

                if ctrl.jump && !ctrl.fire2 {
                    if matches!(self.mode, MotionMode::Parachuting) {
                        // Stop parachuting
                        self.mode = MotionMode::Standing;
                        self.phys.drag = NORMAL_DRAG;
                    } else if terrain::is_solid(ter) && terrain::is_space(ter_above) {
                        // Jump
                        self.phys.add_impulse(Vec2(ctrl.walk * -40000.0, -50000.0));
                        self.mode = MotionMode::Jetpacking;
                    } else if terrain::is_space(ter) && self.jetpack_charge > 0.0 {
                        // Jetpack
                        self.mode = MotionMode::Jetpacking;
                        self.jetpack_charge -= timestep;
                        call_state_method!(*self, lua, "on_jetpack", ctrl.walk);
                    }
                }
            }

            if ctrl.fire1 && self.weapon_cooldown <= 0.0 {
                call_state_method!(*self, lua, "on_shoot");
            }

            if ctrl.fire2
                && terrain::is_space(ter)
                && !matches!(self.mode, MotionMode::Parachuting | MotionMode::Ninjaroping)
            {
                // Activate parachute
                self.mode = MotionMode::Parachuting;
                self.phys.drag = PARACHUTE_DRAG;
            }

            if ctrl.fire3 && !self.fire3_down {
                match self.ninjarope {
                    NinjaRope::Stowed => {
                        self.ninjarope = NinjaRope::Extending {
                            vec: self.aim_vector(1.0, true),
                            length: 1.0,
                        }
                    }
                    _ => {
                        self.ninjarope = NinjaRope::Stowed;
                        if matches!(self.mode, MotionMode::Ninjaroping) {
                            self.mode = MotionMode::Jetpacking;
                        }
                    }
                }
            }
            self.fire3_down = ctrl.fire3;
        }

        if terrain::is_solid(ter) {
            self.jetpack_charge = (self.jetpack_charge + timestep).min(1.0);
        } else if terrain::is_water(ter)
            || matches!(self.mode, MotionMode::Parachuting | MotionMode::Ninjaroping)
        {
            self.jetpack_charge = (self.jetpack_charge + timestep * 0.1).min(1.0);
        }

        if let NinjaRope::Extending { vec, length } = self.ninjarope {
            let newlen = length + NINJAROPE_SPEED * timestep;
            if newlen > NINJAROPE_MAX_LEN {
                self.ninjarope = NinjaRope::Stowed;
            } else {
                let prev = self.pos() + Self::GUN_OFFSET + vec * length;
                let next = self.pos() + Self::GUN_OFFSET + vec * newlen;
                if let TerrainLineHit::Hit(_, pos) = level.terrain_line(LineF(prev, next)) {
                    self.ninjarope = NinjaRope::Attached(Rope::new(self.pos(), pos));
                    self.mode = MotionMode::Ninjaroping;
                    self.phys.drag = NORMAL_DRAG;
                } else {
                    self.ninjarope = NinjaRope::Extending {
                        vec,
                        length: newlen,
                    };
                }
            }
        }

        gameobject_timer!(*self, lua, timestep);
    }

    fn adjust_aim(&mut self, step: f32) {
        self.aim_angle = (self.aim_angle - 180.0 * step).clamp(-90.0, 90.0);
    }

    pub fn touch_ship(&mut self, ship: &mut Ship, lua: &mlua::Lua) {
        get_state_method!(self, lua, "on_touch_ship", (f, scope) => {
            f.call::<Option<bool>>((
                scope.create_userdata_ref_mut(self)?,
                scope.create_userdata_ref_mut(ship)?,
            ))
        });
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        let tex = match self.mode {
            MotionMode::Standing => &self.stand_texture,
            MotionMode::Jetpacking | MotionMode::Ninjaroping => &self.jetpack_texture,
            MotionMode::Walking => &self.walk_texture,
            MotionMode::Swimming => &self.swim_texture,
            MotionMode::Parachuting => &self.parachute_texture,
        };

        match &self.ninjarope {
            NinjaRope::Stowed => {}
            NinjaRope::Extending { vec, length } => {
                Rope::render_rope(
                    self.pos() + Self::GUN_OFFSET,
                    self.pos() + Self::GUN_OFFSET + *vec * *length,
                    renderer,
                    camera_pos,
                );
            }
            NinjaRope::Attached(rope) => {
                rope.render(self.pos() + Self::GUN_OFFSET, renderer, camera_pos);
            }
        }

        let mut opts = RenderOptions {
            dest: match self.mode {
                MotionMode::Swimming => RenderDest::Centered(self.phys.pos - camera_pos),
                _ => RenderDest::BottomCentered(self.phys.pos - camera_pos),
            },
            mode: if self.facing < 0 {
                RenderMode::Mirrored
            } else {
                RenderMode::Normal
            },
            ..Default::default()
        };
        tex.render(renderer, &opts);
        opts.color = Color::player_color(self.player_id);
        tex.render_alt(renderer, TexAlt::Decal, &opts);
    }
}

impl GameObject for Pilot {
    fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    fn pos(&self) -> Vec2 {
        self.phys.pos
    }

    fn radius(&self) -> f32 {
        self.phys.radius
    }
}
