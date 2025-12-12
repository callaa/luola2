use crate::{
    call_state_method,
    game::{
        GameController, PlayerId,
        level::{LEVEL_SCALE, Level, terrain},
        objects::{GameObject, PhysicalObject, TerrainCollisionMode},
    },
    gfx::{AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TexAlt},
    math::Vec2,
};

#[derive(Clone, Debug)]
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
    weapon_cooldown: f32,
}

#[derive(Clone, Debug)]
enum MotionMode {
    Standing,
    Walking,
    Parachuting,
    Jetpacking,
    Swimming,
}

impl mlua::UserData for Pilot {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("is_pilot", true);
        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_get("facing", |_, this| Ok(this.facing));
        fields.add_field_method_get("player", |_, this| Ok(this.player_id));
        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));

        fields.add_field_method_set("weapon_cooldown", |_, this, cooldown| {
            this.weapon_cooldown = cooldown;
            Ok(())
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("aim_vector", |_, this, mag: f32| Ok(this.aim_vector(mag)));
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
                weapon_cooldown: 0.0,
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

    pub fn aim_vector(&self, mag: f32) -> Vec2 {
        if !self.aim_mode
            && let Some(at) = self.auto_target
        {
            (at - self.pos() - Vec2(0.0, -16.0)).normalized() * mag
        } else {
            Vec2::for_angle(self.aim_angle, mag).element_wise_product(Vec2(self.facing as f32, 1.0))
        }
    }

    /// Get the position (in world coordinates) for the targeting reticle
    pub fn aim_target(&self) -> Option<Vec2> {
        if self.aim_mode {
            Some(self.phys.pos + Vec2(0.0, -16.0) + self.aim_vector(60.0))
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
            self.aim_mode = ctrl.fire_secondary;
            self.weapon_cooldown -= timestep;

            let ter_above = level.terrain_at(self.phys.pos - Vec2(0.0, LEVEL_SCALE));
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
                if self.aim_mode {
                    // Aim upwards
                    self.aim_angle = (self.aim_angle - 180.0 * timestep).max(-90.0);
                } else if terrain::is_underwater(ter) {
                    // Swim up
                    self.phys.add_impulse(Vec2(0.0, -2000.0));
                }
            }

            if ctrl.thrust < 0.0 {
                if self.aim_mode {
                    // Aim downwards
                    self.aim_angle = (self.aim_angle + 180.0 * timestep).min(90.0);
                } else if matches!(self.mode, MotionMode::Parachuting) {
                    // Descend faster
                    self.phys.add_impulse(Vec2(0.0, 1000.0));
                } else if terrain::is_water(ter) {
                    // Swim down
                    self.phys.add_impulse(Vec2(0.0, 2000.0));
                }
            }

            if ctrl.jump && !self.aim_mode {
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

            if ctrl.fire_primary && self.weapon_cooldown <= 0.0 {
                call_state_method!(*self, lua, "on_shoot");
            }

            if ctrl.fire_secondary
                && terrain::is_space(ter)
                && !matches!(self.mode, MotionMode::Parachuting)
            {
                // Activate parachute
                self.mode = MotionMode::Parachuting;
                self.phys.drag = PARACHUTE_DRAG;
            }
        }

        if terrain::is_solid(ter) {
            self.jetpack_charge = (self.jetpack_charge + timestep).min(1.0);
        } else if terrain::is_water(ter) || matches!(self.mode, MotionMode::Parachuting) {
            self.jetpack_charge = (self.jetpack_charge + timestep * 0.1).min(1.0);
        }
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        let tex = match self.mode {
            MotionMode::Standing => &self.stand_texture,
            MotionMode::Jetpacking => &self.jetpack_texture,
            MotionMode::Walking => &self.walk_texture,
            MotionMode::Swimming => &self.swim_texture,
            MotionMode::Parachuting => &self.parachute_texture,
        };

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
