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
        Text, TextOutline, Texture, TextureId,
    },
    math::{RectF, Vec2},
};
use core::ops::Deref;

#[derive(Clone, Copy)]
pub enum PlayerHud {
    Ship {
        health: f32,
        ammo: f32,
        cooling_down: bool,
    },
    Pilot {
        jetpack: f32,
        target: Option<Vec2>,
    },
    None,
}

pub struct HudOverlay {
    content: HudOverlayContent,
    pos: Vec2,
    align: HudOverlayAlignment,
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

pub enum HudOverlayAlignment {
    TopLeft,
    Centered,
    StatusArea,
}

impl mlua::FromLua for HudOverlay {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        if let mlua::Value::Table(table) = value {
            let text = table.get::<Option<Text>>("text")?;
            let texture = table.get::<Option<TextureId>>("texture")?;
            let pos: Vec2 = table.get("pos")?;

            let align = if let Some(align) = table.get::<Option<mlua::String>>("align")? {
                match align.as_bytes().deref() {
                    b"topleft" => HudOverlayAlignment::TopLeft,
                    b"center" => HudOverlayAlignment::Centered,
                    b"status" => HudOverlayAlignment::StatusArea,
                    unknown => {
                        return Err(mlua::Error::FromLuaConversionError {
                            from: "string",
                            to: "HudOverlayAlignment".to_string(),
                            message: Some(format!(
                                "unknown alignment: {}",
                                str::from_utf8(unknown).unwrap()
                            )),
                        });
                    }
                }
            } else {
                HudOverlayAlignment::Centered
            };

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
                pos,
                align,
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

        let pos = Vec2(
            renderer.width() as f32 * self.pos.0,
            renderer.height() as f32 * self.pos.1,
        );

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
                    dest: match self.align {
                        HudOverlayAlignment::Centered => RenderTextDest::Centered(pos),
                        HudOverlayAlignment::TopLeft => RenderTextDest::TopLeft(pos),
                        HudOverlayAlignment::StatusArea => RenderTextDest::BottomLeft(Vec2(
                            20.0 + ((renderer.width() - 20) as f32 * 0.61).ceil(),
                            renderer.height() as f32 - 10.0,
                        )),
                    },
                    color: self.color,
                    alpha: a,
                    outline: TextOutline::Outline,
                });
            }
            HudOverlayContent::Texture(id) => {
                let tex = renderer.texture_store().get_texture(*id);
                let w = tex.width() * scale_factor;
                let h = tex.height() * scale_factor;
                let dest = match self.align {
                    HudOverlayAlignment::Centered => {
                        RectF::new(pos.0 - w / 2.0, pos.1 - h / 2.0, w, h)
                    }
                    HudOverlayAlignment::TopLeft => RectF::new(pos.0, pos.1, w, h),
                    HudOverlayAlignment::StatusArea => RectF::new(
                        20.0 + ((renderer.width() - 20) as f32 * 0.61).ceil(),
                        (renderer.height() - 10) as f32 - h,
                        w,
                        h,
                    ),
                };

                tex.render(
                    renderer,
                    &RenderOptions {
                        dest: RenderDest::Rect(dest),
                        color: self.color.unwrap_or(Color::WHITE).with_alpha(a),
                        mode: match self.angle {
                            Some(a) => RenderMode::Rotated(a, false),
                            None => RenderMode::Normal,
                        },
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn age(&mut self, timestep: f32) -> bool {
        self.age += timestep;
        self.age < self.lifetime
    }
}

pub fn draw_hud(renderer: &Renderer, hud: PlayerHud, overlays: &[HudOverlay], camera_pos: Vec2) {
    match hud {
        PlayerHud::Ship {
            health,
            ammo,
            cooling_down,
            ..
        } => draw_ship_hud(renderer, health, ammo, cooling_down),
        PlayerHud::Pilot { jetpack, target } => {
            draw_pilot_hud(renderer, jetpack, target.map(|t| t - camera_pos))
        }
        PlayerHud::None => {}
    }

    for overlay in overlays {
        overlay.draw(renderer);
    }
}

pub fn draw_minimap(renderer: &Renderer, minimap: &Texture, pointers: &[(Color, Vec2)]) {
    let w = minimap.width();
    let h = minimap.height();
    let x = renderer.width() as f32 - 10.0 - w;
    let y = renderer.height() as f32 - 10.0 - h;

    minimap.render(
        renderer,
        &RenderOptions {
            dest: RenderDest::Rect(RectF::new(x, y, w, h)),
            ..Default::default()
        },
    );

    let tex = renderer.texture_store().get_texture(
        renderer
            .texture_store()
            .find_texture("minimap_pointer")
            .expect("minimap_pointer texture should exist"),
    );
    for (color, pointer) in pointers {
        tex.render(
            renderer,
            &RenderOptions {
                dest: RenderDest::Centered(Vec2(
                    x + (pointer.0 * w).round(),
                    y + (pointer.1 * h).round(),
                )),
                color: *color,
                ..Default::default()
            },
        );
    }
}

fn draw_ship_hud(renderer: &Renderer, health: f32, ammo: f32, cooling_down: bool) {
    let barfill = renderer.texture_store().get_texture(
        renderer
            .texture_store()
            .find_texture("bar_fill")
            .expect("bar_fill texture should exist"),
    );
    let barbg = renderer.texture_store().get_texture(
        renderer
            .texture_store()
            .find_texture("bar_bg")
            .expect("bar_bg texture should exist"),
    );

    let w = ((renderer.width() - 20) as f32 * 0.61).ceil();
    let h = barbg.height();
    let x = 10.0;
    let y = renderer.height() as f32 - (h * 2.0) - 10.0;

    let mut opts = RenderOptions {
        dest: RenderDest::Rect(RectF::new(x, y, w, h)),
        mode: RenderMode::NineGrid(1.0),
        ..Default::default()
    };

    // Health bar
    barbg.render(renderer, &opts);

    if health > 0.0 {
        opts.dest = RenderDest::Rect(RectF::new(x, y, w * health, h));
        opts.color = if health > 0.7 {
            Color::new(0.31, 0.38, 0.72)
        } else if health > 0.4 {
            Color::new(0.78, 0.78, 0.0)
        } else {
            Color::new(0.78, 0.0, 0.0)
        };
        barfill.render(renderer, &opts);
    }

    // Ammo bar
    opts.dest = RenderDest::Rect(RectF::new(x, y + h, w, h));
    opts.color = Color::WHITE;
    barbg.render(renderer, &opts);

    if ammo > 0.0 {
        opts.dest = RenderDest::Rect(RectF::new(x, y + h, w * ammo, h));
        opts.color = Color {
            r: 0.72,
            g: 0.76,
            b: 0.76,
            a: if cooling_down { 0.5 } else { 1.0 },
        };
        barfill.render(renderer, &opts);
    }
}

fn draw_pilot_hud(renderer: &Renderer, jetpack: f32, target: Option<Vec2>) {
    let barfill = renderer.texture_store().get_texture(
        renderer
            .texture_store()
            .find_texture("bar_fill")
            .expect("bar_fill texture should exist"),
    );
    let barbg = renderer.texture_store().get_texture(
        renderer
            .texture_store()
            .find_texture("bar_bg")
            .expect("bar_bg texture should exist"),
    );

    let w = ((renderer.width() - 20) as f32 * 0.1).ceil();
    let h = barbg.height();
    let x = 10.0;
    let y = renderer.height() as f32 - (h * 2.0) - 10.0;

    let mut opts = RenderOptions {
        dest: RenderDest::Rect(RectF::new(x, y, w, h)),
        mode: RenderMode::NineGrid(1.0),
        ..Default::default()
    };

    // Jetpack charge bar
    barbg.render(renderer, &opts);

    if jetpack > 0.0 {
        opts.dest = RenderDest::Rect(RectF::new(x, y, w * jetpack, h));
        barfill.render(renderer, &opts);
    }

    // Target reticle
    if let Some(t) = target {
        let tex = renderer.texture_store().get_texture(
            renderer
                .texture_store()
                .find_texture("hud_reticle")
                .expect("hud_reticle texture should exist"),
        );
        tex.render(
            renderer,
            &RenderOptions {
                dest: RenderDest::CenterScaled(t, 2.0),
                ..Default::default()
            },
        );
    }
}
