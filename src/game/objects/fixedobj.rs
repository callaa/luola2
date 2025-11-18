use crate::{
    call_state_method, gameobject_timer,
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
    texture: Option<AnimatedTexture>,
    color: Color,
    state: Option<mlua::Table>,

    /// Object scheduler
    timer: Option<f32>,
    timer_accumulator: f32,
}

impl mlua::FromLua for FixedObject {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let texture = table
                .get::<Option<TextureId>>("texture")?
                .map(AnimatedTexture::new);
            Ok(FixedObject {
                id: table.get("id")?,
                pos: table.get("pos")?,
                radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                texture,
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                destroyed: false,
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

        fields.add_field_method_get("texture", |_, this| {
            Ok(this.texture.as_ref().map(|t| t.id()))
        });
        fields.add_field_method_set("texture", |_, this, t: Option<TextureId>| {
            this.texture = t.map(AnimatedTexture::new);
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
            call_state_method!(*self, lua, "on_destroy");
        }
    }

    pub fn step_mut(&mut self, lua: &mlua::Lua, timestep: f32) {
        if let Some(tex) = &mut self.texture {
            tex.step(timestep);
        }

        gameobject_timer!(*self, lua, timestep);
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        if let Some(tex) = &self.texture {
            let options = RenderOptions {
                dest: RenderDest::Centered(self.pos - camera_pos),
                ..Default::default()
            };
            tex.render(renderer, &options);
        }
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
