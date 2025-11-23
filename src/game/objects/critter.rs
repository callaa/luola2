use crate::{
    call_state_method,
    game::{
        PlayerId,
        level::{LEVEL_SCALE, Level, terrain},
        objects::{GameObject, PhysicalObject, Projectile, Rope, TerrainCollisionMode},
    },
    gameobject_timer, get_state_method,
    gfx::{
        AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TexAlt, TextureId,
    },
    math::Vec2,
};

#[derive(Clone, Debug)]
pub struct Critter {
    phys: PhysicalObject,

    /// Hostile critters will not attack their owners
    owner: PlayerId,

    /// Unique ID (useful for distinguishing self from others in a flock)
    id: u32,

    /// Object flagged for destruction
    destroyed: bool,

    /// If false, touch ground callback will be called when critter touches water
    waterproof: bool,

    /// Walking direction (0 for no walk)
    walking: i8,

    /// Direction to face if not currently walking (applies to flippable sprites)
    /// If zero, phys.vel.0 is used
    facing: i8,

    /// In action state (action_texture animation is run)
    action: bool,

    /// How many seconds to take one step
    walkspeed: f32,

    /// Timer for taking the next step
    step_timer: f32,

    /// Extra state for scripting.
    /// Most critter state lives here.
    state: Option<mlua::Table>,

    /// Rope for attaching to things
    /// Used by spiders.
    rope: Option<Rope>,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,

    texture: AnimatedTexture,

    /// Special action animation
    /// on_action_complete callback will be called when action animation completes
    action_texture: Option<AnimatedTexture>,
}

impl mlua::UserData for Critter {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("is_critter", true);

        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_set("vel", |_, this, v: Vec2| {
            this.phys.vel = v;
            Ok(())
        });
        fields.add_field_method_set("drag", |_, this, d: f32| {
            this.phys.drag = d;
            Ok(())
        });

        fields.add_field_method_get("action", |_, this| Ok(this.action));
        fields.add_field_method_set("action", |_, this, a: bool| {
            this.action = a;
            Ok(())
        });

        fields.add_field_method_get("walking", |_, this| Ok(this.walking));
        fields.add_field_method_set("walking", |_, this, dir: i8| {
            this.walking = dir;
            Ok(())
        });
        fields.add_field_method_get("facing", |_, this| Ok(this.facing));
        fields.add_field_method_set("facing", |_, this, dir: i8| {
            this.facing = dir;
            Ok(())
        });
        fields.add_field_method_get("rope_attached", |_, this| Ok(this.rope.is_some()));
        fields.add_field_method_get("rope_length", |_, this| {
            Ok(match &this.rope {
                Some(r) => r.length(),
                None => 0.0,
            })
        });
        fields.add_field_method_get("texture", |_, this| Ok(this.texture.id()));
        fields.add_field_method_set("texture", |_, this, t: TextureId| {
            this.texture = AnimatedTexture::new(t);
            Ok(())
        });
        fields.add_field_method_get("id", |_, this| Ok(this.id));
        fields.add_field_method_get("owner", |_, this| Ok(this.owner));
        fields.add_field_method_get("timer", |_, this| Ok(this.timer));
        fields.add_field_method_set("timer", |_, this, timeout: Option<f32>| {
            this.timer = timeout;
            Ok(())
        });

        fields.add_field_method_get("state", |_, this| Ok(this.state.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("destroy", |lua, this, _: ()| {
            this.destroy(lua);
            Ok(())
        });

        methods.add_method_mut("impulse", |_, this, v: Vec2| {
            this.phys.add_impulse(v);
            Ok(())
        });

        methods.add_method_mut("attach_rope", |_, this, pos: Vec2| {
            this.rope = Some(Rope::new(this.phys.pos, pos));
            Ok(())
        });

        methods.add_method_mut("detach_rope", |_, this, _: ()| {
            Ok(if this.rope.is_some() {
                this.rope = None;
                true
            } else {
                false
            })
        });

        methods.add_method_mut("climb_rope", |_, this, d: f32| {
            if let Some(r) = &mut this.rope {
                r.adjust(d);
            }
            Ok(())
        });
    }
}

impl mlua::FromLua for Critter {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            Ok(Critter {
                phys: PhysicalObject {
                    pos: table.get("pos")?,
                    vel: table.get::<Option<Vec2>>("vel")?.unwrap_or_default(),
                    imass: 1.0 / table.get::<Option<f32>>("mass")?.unwrap_or(1000.0),
                    radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                    drag: table.get::<Option<f32>>("drag")?.unwrap_or(0.025),
                    impulse: Vec2::ZERO,
                    terrain_collision_mode: TerrainCollisionMode::Simple,
                },
                id: table.get::<Option<u32>>("id")?.unwrap_or(0),
                owner: table.get::<Option<i32>>("owner")?.unwrap_or(0),
                waterproof: table.get::<Option<bool>>("waterproof")?.unwrap_or(true),
                walking: table.get::<Option<i8>>("walking")?.unwrap_or(0),
                facing: 0,
                walkspeed: table.get::<Option<f32>>("walkspeed")?.unwrap_or(0.03),
                step_timer: 0.0,
                rope: None,
                action: false,
                state: table.get("state")?,
                texture: AnimatedTexture::new(table.get("texture")?),
                action_texture: table
                    .get::<Option<TextureId>>("action_texture")?
                    .map(|id| AnimatedTexture::new(id)),
                destroyed: false,
                timer: table.get("timer")?,
                timer_accumulator: 0.0,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Critter".to_owned(),
                message: Some("expected a table describing a Critter".to_string()),
            })
        }
    }
}

impl Critter {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn physics(&self) -> &PhysicalObject {
        &self.phys
    }

    pub fn physics_mut(&mut self) -> &mut PhysicalObject {
        &mut self.phys
    }

    pub fn owner(&self) -> PlayerId {
        self.owner
    }

    pub fn destroy(&mut self, lua: &mlua::Lua) {
        if !self.destroyed {
            self.destroyed = true;
            call_state_method!(*self, lua, "on_destroy");
        }
    }

    // Take a step to the right or left
    fn walk(&mut self, level: &Level) -> (Vec2, bool) {
        let new_x = self.phys.pos.0 + (self.walking as f32 * LEVEL_SCALE);

        const MAX_SLOPE: i32 = 5;

        for i in 0..MAX_SLOPE {
            // uphill
            let new_pos = Vec2(new_x, self.phys.pos.1 + i as f32 * LEVEL_SCALE);
            let ter_at = level.terrain_at(new_pos);
            if terrain::is_solid(ter_at) && !terrain::can_walk_through(ter_at) {
                // pixel above must be free
                let above = new_pos - Vec2(0.0, LEVEL_SCALE);
                if terrain::can_walk_through(level.terrain_at(above)) {
                    return (above, false);
                }
            }

            // downhill
            let new_pos = Vec2(new_x, self.phys.pos.1 - i as f32 * LEVEL_SCALE);
            let ter_at = level.terrain_at(new_pos);
            if terrain::is_solid(ter_at) && !terrain::can_walk_through(ter_at) {
                // pixel above must be free
                let above = new_pos - Vec2(0.0, LEVEL_SCALE);
                if terrain::can_walk_through(level.terrain_at(above)) {
                    return (above, false);
                }
            }
        }

        (
            self.pos(),
            !terrain::can_walk_through(level.terrain_at(self.phys.pos + Vec2(0.0, LEVEL_SCALE))),
        )
    }

    /// Perform a simulation step and return a new copy of this critter
    pub fn step(&self, level: &Level, lua: &mlua::Lua, timestep: f32) -> Self {
        let mut critter = self.clone();

        let (_, ter) = critter.phys.step(level, timestep);

        if let Some(rope) = &self.rope {
            rope.physics_step(&mut critter.phys);
        }

        if terrain::is_solid(ter) || (terrain::is_water(ter) && !critter.waterproof) {
            call_state_method!(critter, lua, "on_touch_ground", ter);
        }

        if critter.walking != 0 {
            if self.step_timer <= 0.0 {
                let (pos, stopped) = critter.walk(level);
                critter.phys.pos = pos;
                critter.step_timer = self.walkspeed;
                if stopped {
                    call_state_method!(critter, lua, "on_touch_ledge");
                }
            } else {
                critter.step_timer -= timestep;
            }
        }

        if critter.action
            && let Some(t) = &mut critter.action_texture
        {
            if t.step(timestep) {
                critter.action = false;
                call_state_method!(critter, lua, "on_action_complete");
            }
        } else {
            critter.texture.step(timestep);
        }

        gameobject_timer!(critter, lua, timestep);

        critter
    }

    fn need_flip_tex(&self) -> bool {
        if self.walking != 0 {
            self.walking < 0
        } else if self.facing != 0 {
            self.facing < 0
        } else {
            self.phys.vel.0 < 0.0
        }
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        if let Some(rope) = &self.rope {
            rope.render(self.phys.pos, renderer, camera_pos);
        }

        let mut options = RenderOptions {
            dest: RenderDest::Centered(self.phys.pos - camera_pos),
            mode: if self.texture.id().flippable() && self.need_flip_tex() {
                RenderMode::Mirrored
            } else {
                RenderMode::Normal
            },
            ..Default::default()
        };
        let t = if self.action
            && let Some(t) = &self.action_texture
        {
            t
        } else {
            &self.texture
        };
        t.render(renderer, &options);

        if self.owner != 0 {
            options.color = Color::player_color(self.owner);
            t.render_alt(renderer, TexAlt::Decal, &options);
        }
    }

    /// Execute bullet hit callback.
    /// Returns true if bullet impact callback should be processed as usual too.
    pub fn bullet_hit(&mut self, bullet: &mut Projectile, lua: &mlua::Lua) -> bool {
        get_state_method!(self, lua, "on_bullet_hit", (f, scope) => {
            f.call::<Option<bool>>((
                scope.create_userdata_ref_mut(self)?,
                scope.create_userdata_ref_mut(bullet)?,
            ))
        })
        .unwrap_or(true)
    }
}

impl GameObject for Critter {
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
