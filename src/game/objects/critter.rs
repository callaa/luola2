use crate::{
    game::{
        PlayerId,
        level::{LEVEL_SCALE, Level, terrain},
        objects::{GameObject, PhysicalObject, Projectile, Rope, TerrainCollisionMode},
    },
    gfx::{
        AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TexAlt, TextureId,
    },
    math::Vec2,
};

static mut LAST_CRITTER_ID: u32 = 0;

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

    /// How many seconds to take one step
    walkspeed: f32,

    /// Timer for taking the next step
    step_timer: f32,

    /// Extra state for scripting.
    /// Most critter state lives here.
    state: mlua::Table,

    /// Rope for attaching to things
    /// Used by spiders.
    rope: Option<Rope>,

    /// Callback to handle bullet hits. (Typically critters are one shotted by any projectile)
    /// The callback may return "false" to indicate it has performed special handling
    /// and the bullet's normal impact handler should not be run.
    /// function (this, bullet) -> Option<bool>
    on_bullet_hit: mlua::Function,

    on_touch_ground: Option<mlua::Function>,

    /// Called when a walking critter can't walk any further
    on_touch_ledge: Option<mlua::Function>,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,

    texture: AnimatedTexture,
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
        methods.add_method_mut("destroy", |_, this, _: ()| {
            this.destroyed = true;
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
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        unsafe {
            // we're not multithreaded so fine for now
            LAST_CRITTER_ID += 1;
        }

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
                id: unsafe { LAST_CRITTER_ID },
                owner: table.get::<Option<i32>>("owner")?.unwrap_or(0),
                waterproof: table.get::<Option<bool>>("waterproof")?.unwrap_or(true),
                walking: table.get::<Option<i8>>("walking")?.unwrap_or(0),
                facing: 0,
                walkspeed: table.get::<Option<f32>>("walkspeed")?.unwrap_or(0.03),
                step_timer: 0.0,
                rope: None,
                state: table
                    .get::<Option<mlua::Table>>("state")?
                    .unwrap_or_else(|| lua.create_table().unwrap()),
                texture: AnimatedTexture::new(table.get("texture")?),
                on_bullet_hit: table.get("on_bullet_hit")?,
                on_touch_ledge: table.get("on_touch_ledge")?,
                on_touch_ground: table.get("on_touch_ground")?,
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

    // Take a step to the right or left
    fn walk(&mut self, level: &Level) -> (Vec2, bool) {
        let new_x = self.phys.pos.0 + (self.walking as f32 * LEVEL_SCALE);

        const MAX_SLOPE: i32 = 5;

        for i in 0..MAX_SLOPE {
            // uphill
            let new_pos = Vec2(new_x, self.phys.pos.1 + i as f32 * LEVEL_SCALE);
            if terrain::is_solid(level.terrain_at(new_pos)) {
                // pixel above must be free
                let new_pos = new_pos - Vec2(0.0, LEVEL_SCALE);
                if terrain::can_walk_through(level.terrain_at(new_pos)) {
                    return (new_pos, false);
                }
            }

            // downhill
            let new_pos = Vec2(new_x, self.phys.pos.1 - i as f32 * LEVEL_SCALE);
            if terrain::is_solid(level.terrain_at(new_pos)) {
                // pixel above must be free
                let new_pos = new_pos - Vec2(0.0, LEVEL_SCALE);
                if terrain::can_walk_through(level.terrain_at(new_pos)) {
                    return (new_pos, false);
                }
            }
        }

        (self.pos(), true)
    }

    /// Perform a simulation step and return a new copy of this critter
    pub fn step(&self, level: &Level, lua: &mlua::Lua, timestep: f32) -> Self {
        let mut critter = self.clone();

        let (_, ter) = critter.phys.step(level, timestep);

        if let Some(rope) = &self.rope {
            rope.physics_step(&mut critter.phys);
        }

        if (terrain::is_solid(ter) || (terrain::is_water(ter) && !critter.waterproof))
            && let Some(callback) = critter.on_touch_ground.clone()
            && let Err(err) = lua.scope(|scope| {
                callback.call::<()>((scope.create_userdata_ref_mut(&mut critter)?, ter))
            })
        {
            log::error!("Critter on_touch_ground: {err}");
            critter.timer = None;
        }

        if critter.walking != 0 {
            if self.step_timer <= 0.0 {
                let (pos, stopped) = critter.walk(level);
                critter.phys.pos = pos;
                critter.step_timer = self.walkspeed;
                if stopped
                    && let Some(callback) = critter.on_touch_ledge.clone()
                    && let Err(err) = lua.scope(|scope| {
                        callback.call::<()>(scope.create_userdata_ref_mut(&mut critter)?)
                    })
                {
                    log::error!("Critter on_touch_ledge: {err}");
                    critter.timer = None;
                }
            } else {
                critter.step_timer -= timestep;
            }
        }

        critter.texture.step(timestep);

        if let Some(timer) = critter.timer.as_mut() {
            *timer -= timestep;
            critter.timer_accumulator += timestep;
            let acc = critter.timer_accumulator;

            if *timer <= 0.0 {
                critter.timer_accumulator = 0.0;
                match lua.scope(|scope| {
                    lua.globals()
                        .get::<mlua::Function>("luola_on_object_timer")?
                        .call::<Option<f32>>((scope.create_userdata_ref_mut(&mut critter)?, acc))
                }) {
                    Ok(rerun) => {
                        critter.timer = rerun;
                    }
                    Err(err) => {
                        log::error!("Critter timer : {err}");
                        critter.timer = None;
                    }
                };
            }
        }

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
        self.texture.render(renderer, &options);

        if self.owner != 0 {
            options.color = Color::player_color(self.owner);
            self.texture.render_alt(renderer, TexAlt::Decal, &options);
        }
    }

    /// Execute bullet hit callback.
    /// Returns true if bullet impact callback should be processed as usual too.
    pub fn bullet_hit(&mut self, bullet: &mut Projectile, lua: &mlua::Lua) -> bool {
        let cb = self.on_bullet_hit.clone();
        match lua.scope(|scope| {
            cb.call::<Option<bool>>((
                scope.create_userdata_ref_mut(self)?,
                scope.create_userdata_ref_mut(bullet)?,
            ))
        }) {
            Ok(ret) => ret.unwrap_or(true),
            Err(err) => {
                log::error!("Critter bullet hit callback: {err}");
                true
            }
        }
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
