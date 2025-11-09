use crate::{
    gfx::{AnimatedTexture, Color, RenderDest, RenderOptions, Renderer, TextureId},
    math::Vec2,
};

use super::GameObject;

/**
 * Special game objects that are fixed in place
 */
#[derive(Clone, Debug)]
pub struct FixedObject {
    id: i32,
    pos: Vec2,
    radius: f32,
    destroyed: bool,
    texture: AnimatedTexture,
    color: Color,
    state: mlua::Table,

    on_destroy: Option<mlua::Function>,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,
}

impl mlua::FromLua for FixedObject {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            Ok(FixedObject {
                id: table.get("id")?,
                pos: table.get("pos")?,
                radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                texture: AnimatedTexture::new(table.get("texture")?),
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                destroyed: false,
                on_destroy: table.get("on_destroy")?,
                state: table.get("state")?,
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

impl mlua::UserData for FixedObject {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("pos", |_, this| Ok(this.pos));

        fields.add_field_method_get("texture", |_, this| Ok(this.texture.id()));
        fields.add_field_method_set("texture", |_, this, t: TextureId| {
            this.texture = AnimatedTexture::new(t);
            Ok(())
        });
        fields.add_field_method_get("id", |_, this| Ok(this.id));
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
    }
}

impl FixedObject {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn destroy(&mut self, lua: &mlua::Lua) {
        if !self.destroyed {
            self.destroyed = true;

            if let Some(callback) = self.on_destroy.as_ref()
                && let Err(err) =
                    lua.scope(|scope| callback.call::<()>(scope.create_userdata_ref(self)?))
            {
                log::error!("FixedObject on_destroy: {err}");
            }
        }
    }

    pub fn step_mut(&mut self, lua: &mlua::Lua, timestep: f32) {
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
                        log::error!("FixedObject timer : {err}");
                        self.timer = None;
                    }
                };
            }
        }
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        let options = RenderOptions {
            dest: RenderDest::Centered(self.pos - camera_pos),
            ..Default::default()
        };
        self.texture.render(renderer, &options);
    }
}

impl GameObject for FixedObject {
    fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    fn pos(&self) -> crate::math::Vec2 {
        self.pos
    }

    fn radius(&self) -> f32 {
        self.radius
    }
}
