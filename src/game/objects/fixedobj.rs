use crate::{
    call_state_method,
    game::objects::PhysicalObject,
    gameobject_timer,
    gfx::{AnimatedTexture, Color, RenderDest, RenderMode, RenderOptions, Renderer, TextureId},
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
    action_texture: Option<AnimatedTexture>,
    color: Color,
    state: Option<mlua::Table>,
    action: bool,
    angle: f32,

    /// Flag that is set when the object is moved via scripting
    moved: bool,

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
            let action_texture = table
                .get::<Option<TextureId>>("action_texture")?
                .map(AnimatedTexture::new);
            Ok(FixedObject {
                id: table.get("id")?,
                pos: table.get("pos")?,
                radius: table.get::<Option<f32>>("radius")?.unwrap_or(1.0),
                texture,
                action_texture,
                action: false,
                color: Color::from_argb_u32(
                    table.get::<Option<u32>>("color")?.unwrap_or(0xffffffff),
                ),
                angle: table.get::<Option<f32>>("angle")?.unwrap_or_default(),
                destroyed: false,
                moved: false,
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
        fields.add_field_method_set("pos", |_, this, p: Vec2| {
            this.pos = p;
            this.moved = true;
            Ok(())
        });

        fields.add_field_method_get("angle", |_, this| Ok(this.angle));
        fields.add_field_method_set("angle", |_, this, a: f32| {
            this.angle = a;
            Ok(())
        });

        fields.add_field_method_get("texture", |_, this| {
            Ok(this.texture.as_ref().map(|t| t.id()))
        });
        fields.add_field_method_set("texture", |_, this, t: Option<TextureId>| {
            this.texture = t.map(AnimatedTexture::new);
            Ok(())
        });
        fields.add_field_method_get("action_texture", |_, this| {
            Ok(this.action_texture.as_ref().map(|t| t.id()))
        });
        fields.add_field_method_set("action_texture", |_, this, t: Option<TextureId>| {
            this.action_texture = t.map(AnimatedTexture::new);
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
        methods.add_method_mut("action", |_lua, this, _: ()| {
            this.action = true;
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

    // Check overlap with a physical object
    pub fn check_overlap(&self, obj: &PhysicalObject) -> bool {
        let distv = self.pos - obj.pos;
        let dd = distv.dot(distv);
        let r = self.radius + obj.radius;

        dd <= r * r
    }

    pub fn step_mut(&mut self, lua: &mlua::Lua, timestep: f32) -> bool {
        if self.action {
            let complete = if let Some(tex) = &mut self.action_texture {
                tex.step(timestep)
            } else {
                true
            };

            if complete {
                self.action = false;
                call_state_method!(*self, lua, "on_action_complete");
            }
        } else if let Some(tex) = &mut self.texture {
            tex.step(timestep);
        }

        gameobject_timer!(*self, lua, timestep);

        let changed = self.moved | self.destroyed;
        self.moved = false;

        changed
    }

    pub fn render(&self, renderer: &Renderer, camera_pos: Vec2) {
        let tex = if self.action {
            &self.action_texture
        } else {
            &self.texture
        };
        if let Some(tex) = tex {
            let options = RenderOptions {
                dest: RenderDest::Centered(self.pos - camera_pos),
                mode: if tex.id().needs_rotation() {
                    RenderMode::Rotated(self.angle, false)
                } else {
                    RenderMode::Normal
                },
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
