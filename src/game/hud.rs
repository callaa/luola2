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

use crate::{
    gfx::{
        Color, RenderDest, RenderMode, RenderOptions, RenderTextDest, RenderTextOptions, Renderer,
        Text, TextOutline, TextureId,
    },
    math::{RectF, Vec2},
};

#[derive(Clone, Copy)]
pub enum PlayerHud {
    Ship { health: f32, ammo: f32 },
    None,
}

pub struct HudOverlay {
    content: HudOverlayContent,
    pos: HudOverlayPosition,
    /// Scale factor for texture. 1.0 means width of the viewport
    scale: Option<f32>,
    /// Rotation angle (currently only works for textures)
    angle: Option<f32>,
    color: Option<Color>,
    lifetime: f32,
    fadein: f32,
    fadeout: f32,
    age: f32,
}

pub enum HudOverlayContent {
    Text(Text),
    Texture(TextureId),
}

pub enum HudOverlayPosition {
    Centered(Vec2),
}

impl mlua::FromLua for HudOverlay {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let text = table.get::<Option<Text>>("text")?;
            let texture = table.get::<Option<TextureId>>("texture")?;
            let posv: Vec2 = table.get("pos")?;

            let content = match (text, texture) {
                (Some(t), None) => HudOverlayContent::Text(t),
                (None, Some(id)) => HudOverlayContent::Texture(id),
                _ => {
                    return Err(mlua::Error::FromLuaConversionError {
                        from: "table",
                        to: "HudOverlay".to_string(),
                        message: Some(
                            "both texture and text cannot be set at the same time".to_string(),
                        ),
                    });
                }
            };

            Ok(Self {
                content,
                pos: HudOverlayPosition::Centered(posv),
                scale: table.get("scale")?,
                angle: table.get("angle")?,
                color: table.get::<Option<u32>>("color")?.map(Color::from_argb_u32),
                lifetime: table.get::<Option<f32>>("lifetime")?.unwrap_or(0.0),
                fadein: table.get::<Option<f32>>("fadein")?.unwrap_or(0.0),
                fadeout: table.get::<Option<f32>>("fadeout")?.unwrap_or(0.0),
                age: 0.0,
            })
        } else {
            Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HudOverlay".to_string(),
                message: Some("expected a table describing a HUD overlay".to_string()),
            })
        }
    }
}

impl HudOverlay {
    fn draw(&self, renderer: &Renderer) {
        let a = if self.fadein > 0.0 && self.age < self.fadein {
            self.age / self.fadein
        } else if self.fadeout > 0.0 && (self.lifetime - self.age) < self.fadeout {
            ((self.lifetime - self.age) / self.fadeout).max(0.0)
        } else {
            1.0
        };

        let pos = match self.pos {
            HudOverlayPosition::Centered(p) => Vec2(
                renderer.width() as f32 * p.0,
                renderer.height() as f32 * p.1,
            ),
        };

        let scale_factor = if let Some(s) = self.scale
            && let HudOverlayContent::Texture(id) = self.content
        {
            let w = renderer.texture_store().get_texture(id).width();

            renderer.width() as f32 / w * s
        } else {
            1.0
        };

        match &self.content {
            HudOverlayContent::Text(t) => {
                t.render(&RenderTextOptions {
                    dest: RenderTextDest::Centered(pos),
                    color: self.color,
                    alpha: a,
                    outline: TextOutline::Outline,
                });
            }
            HudOverlayContent::Texture(tex) => renderer.texture_store().get_texture(*tex).render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::CenterScaled(pos, scale_factor),
                    color: self.color.unwrap_or(Color::WHITE).with_alpha(a),
                    mode: match self.angle {
                        Some(a) => RenderMode::Rotated(a, false),
                        None => RenderMode::Normal,
                    },
                    ..Default::default()
                },
            ),
        }
    }

    pub fn age(&mut self, timestep: f32) -> bool {
        self.age += timestep;
        self.age < self.lifetime
    }
}

pub fn draw_hud(renderer: &Renderer, hud: PlayerHud, overlays: &[HudOverlay]) {
    match hud {
        PlayerHud::Ship { health, ammo } => draw_ship_hud(renderer, health, ammo),
        PlayerHud::None => {}
    }

    for overlay in overlays {
        overlay.draw(renderer);
    }
}

fn draw_ship_hud(renderer: &Renderer, health: f32, ammo: f32) {
    let bar_height = 3.0 + 4.0 * 2.0;
    let bar_width = renderer.width() as f32 - bar_height * 2.0;

    let bar_rect = RectF::new(
        ((renderer.width() as f32 - bar_width) / 2.0).floor(),
        renderer.height() as f32 - bar_height * 2.0,
        bar_width,
        bar_height,
    );
    renderer.draw_filled_rectangle(bar_rect, &Color::new(0.1, 0.1, 0.1));

    let health_rect = RectF::new(
        bar_rect.x() + 1.0,
        bar_rect.y() + 1.0,
        (bar_rect.w() - 2.0) * health,
        4.0,
    );

    let health_color = if health > 0.5 {
        Color::new(0.31, 0.38, 0.72)
    } else if health > 0.2 {
        Color::new(0.78, 0.78, 0.0)
    } else {
        Color::new(0.78, 0.0, 0.0)
    };

    renderer.draw_filled_rectangle(health_rect, &health_color);

    let ammo_rect = RectF::new(
        bar_rect.x() + 1.0,
        bar_rect.y() + 2.0 + 4.0,
        (bar_rect.w() - 2.0) * ammo,
        4.0,
    );

    renderer.draw_filled_rectangle(ammo_rect, &Color::new(0.72, 0.76, 0.76));
}
