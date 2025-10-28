use crate::{
    game::{
        level::{Level, terrain},
        objects::{GameObject, PhysicalObject, Projectile, TerrainCollisionMode},
    },
    gfx::{AnimatedTexture, RenderDest, RenderOptions, Renderer},
    math::Vec2,
};

static mut LAST_CRITTER_ID: u32 = 0;

#[derive(Clone, Debug)]
pub struct Critter {
    phys: PhysicalObject,

    /// Hostile critters will not attack their owners
    owner: i32,

    /// Unique ID (useful for distinguishing self from others in a flock)
    id: u32,

    /// Object flagged for destruction
    destroyed: bool,

    /// Extra state for scripting.
    /// Most critter state lives here.
    state: mlua::Table,

    /// Callback to handle bullet hits. (Typically critters are one shotted by any projectile)
    /// The callback may return "false" to indicate it has performed special handling
    /// and the bullet's normal impact handler should not be run.
    /// function (this, bullet) -> Option<bool>
    on_bullet_hit: mlua::Function,

    on_touch_ground: Option<mlua::Function>,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,

    texture: AnimatedTexture,
}

impl mlua::UserData for Critter {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("pos", |_, this| Ok(this.phys.pos));
        fields.add_field_method_get("vel", |_, this| Ok(this.phys.vel));
        fields.add_field_method_set("vel", |_, this, v: Vec2| {
            this.phys.vel = v;
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
                state: table
                    .get::<Option<mlua::Table>>("state")?
                    .unwrap_or_else(|| lua.create_table().unwrap()),
                texture: AnimatedTexture::new(table.get("texture")?),
                on_bullet_hit: table.get("on_bullet_hit")?,
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

    /// Perform a simulation step and return a new copy of this critter
    pub fn step(&self, level: &Level, lua: &mlua::Lua, timestep: f32) -> Self {
        let mut critter = self.clone();

        let ter = critter.phys.step(level, timestep);

        if terrain::is_solid(ter)
            && let Some(callback) = critter.on_touch_ground.clone()
        {
            if let Err(err) =
                lua.scope(|scope| callback.call::<()>(scope.create_userdata_ref_mut(&mut critter)?))
            {
                log::error!("Critter on_touch_ground: {err}");
                critter.timer = None;
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

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        self.texture.render(
            renderer,
            &RenderOptions {
                dest: RenderDest::Centered(self.phys.pos - camera_pos),
                ..Default::default()
            },
        );
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
