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

use std::{ffi::c_int, path::PathBuf, ptr::null_mut};

use anyhow::Result;
use sdl3_ttf_sys::ttf::{
    TTF_CloseFont, TTF_CopyFont, TTF_CreateText, TTF_DestroyText, TTF_DrawRendererText, TTF_Font,
    TTF_GetFontOutline, TTF_GetTextFont, TTF_GetTextSize, TTF_OpenFont, TTF_SetFontOutline,
    TTF_SetTextColorFloat, TTF_SetTextString, TTF_SetTextWrapWidth, TTF_Text,
};

use crate::{
    fs::pathbuf_to_cstring,
    gfx::{Color, Renderer, SdlError},
    math::Vec2,
};

pub struct Font {
    font: *mut TTF_Font,
    outline_font: *mut TTF_Font,
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            TTF_CloseFont(self.font);
        }
    }
}

pub struct Text {
    text: *mut TTF_Text,
    outline: *mut TTF_Text,
    default_color: Color,
    default_outline: Color,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
pub enum RenderTextDest {
    TopLeft(Vec2),
    TopRight(Vec2),
    TopCenter(Vec2),
    BottomLeft(Vec2),
    BottomCenter(Vec2),
    Centered(Vec2),
}

#[derive(Clone)]
pub enum TextOutline {
    /// No outline
    None,

    /// Draw outline (if font has outline)
    Outline,

    /// Use outline as a drop shadow
    Shadow,
}

#[derive(Clone)]
pub struct RenderTextOptions {
    pub dest: RenderTextDest,
    pub color: Option<Color>,
    pub alpha: f32, // alpha modifier, color alpha is multiplied with this
    pub outline: TextOutline,
}

impl Default for RenderTextOptions {
    fn default() -> Self {
        Self {
            dest: RenderTextDest::TopLeft(Vec2(0.0, 0.0)),
            color: None,
            alpha: 1.0,
            outline: TextOutline::None,
        }
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe {
            TTF_DestroyText(self.text);
            TTF_DestroyText(self.outline);
        }
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        let font = unsafe { TTF_CopyFont(self.font) };
        if font.is_null() {
            panic!("Font copy failed!");
        }
        let outline_font = if self.outline_font.is_null() {
            null_mut()
        } else {
            let font2 = unsafe { TTF_CopyFont(self.outline_font) };
            if font2.is_null() {
                panic!("Outline font copy failed!");
            }
            font2
        };
        Self { font, outline_font }
    }
}

impl mlua::UserData for Text {}

impl mlua::FromLua for Text {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.take()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Text".to_owned(),
                message: Some("expected Text".to_string()),
            }),
        }
    }
}

impl Font {
    pub fn from_file(path: PathBuf, ptsize: f32, outline: i32) -> Result<Font> {
        let path = pathbuf_to_cstring(path)?;
        let font = unsafe { TTF_OpenFont(path.as_ptr(), ptsize) };
        if font.is_null() {
            return Err(SdlError::get_error("Couldn't open font").into());
        }

        let outline_font = if outline > 0 {
            let font2 = unsafe { TTF_CopyFont(font) };
            unsafe {
                TTF_SetFontOutline(font2, outline);
            }
            font2
        } else {
            null_mut()
        };

        Ok(Self { font, outline_font })
    }

    pub fn create_text(&self, renderer: &Renderer, string: &str) -> Result<Text> {
        // Note: the text object retains a pointer to the textengine (which points to the renderer)
        // even though we don't model the lifetime, this should be OK since the renderer
        // we create is effectively 'static
        let text = unsafe {
            TTF_CreateText(
                renderer.textengine,
                self.font,
                string.as_ptr() as *const i8,
                string.len(),
            )
        };
        if text.is_null() {
            return Err(SdlError::get_error("Couldn't create text").into());
        }

        let outline = if self.outline_font.is_null() {
            null_mut()
        } else {
            let otext = unsafe {
                TTF_CreateText(
                    renderer.textengine,
                    self.outline_font,
                    string.as_ptr() as *const i8,
                    string.len(),
                )
            };
            if otext.is_null() {
                return Err(SdlError::get_error("Couldn't create outline text").into());
            }
            otext
        };

        let mut width: c_int = 0;
        let mut height: c_int = 0;

        unsafe {
            TTF_GetTextSize(
                if outline.is_null() { text } else { outline },
                &mut width,
                &mut height,
            );
        }

        Ok(Text {
            text,
            outline,
            default_color: Color::WHITE,
            default_outline: Color::BLACK,
            width: width as f32,
            height: height as f32,
        })
    }
}

impl Text {
    pub fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn with_wrapwidth(mut self, width: i32) -> Self {
        self.set_wrapwidth(width);
        self
    }

    pub fn set_wrapwidth(&mut self, wrap_width: i32) {
        let mut width: c_int = 0;
        let mut height: c_int = 0;

        unsafe {
            TTF_SetTextWrapWidth(self.text, wrap_width);
            if !self.outline.is_null() {
                TTF_SetTextWrapWidth(self.outline, wrap_width);
            }
            TTF_GetTextSize(
                if self.outline.is_null() {
                    self.text
                } else {
                    self.outline
                },
                &mut width,
                &mut height,
            );
        }

        self.width = width as f32;
        self.height = height as f32;
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.default_color = color;
        self
    }

    pub fn with_outline_color(mut self, color: Color) -> Self {
        self.default_outline = color;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        let mut width: c_int = 0;
        let mut height: c_int = 0;

        unsafe {
            TTF_SetTextString(self.text, text.as_ptr() as *const i8, text.len());
            if !self.outline.is_null() {
                TTF_SetTextString(self.outline, text.as_ptr() as *const i8, text.len());
                TTF_GetTextSize(self.outline, &mut width, &mut height);
            } else {
                TTF_GetTextSize(self.text, &mut width, &mut height);
            }
        }
        self.width = width as f32;
        self.height = height as f32;
    }

    pub fn set_default_color(&mut self, color: Color) {
        self.default_color = color;
    }

    pub fn render(&self, opts: &RenderTextOptions) {
        let pos = match opts.dest {
            RenderTextDest::TopLeft(p) => p,
            RenderTextDest::TopRight(p) => p - Vec2(self.width, 0.0),
            RenderTextDest::Centered(p) => p - Vec2(self.width / 2.0, self.height / 2.0),
            RenderTextDest::TopCenter(p) => p - Vec2(self.width / 2.0, 0.0),
            RenderTextDest::BottomLeft(p) => p - Vec2(0.0, self.height),
            RenderTextDest::BottomCenter(p) => p - Vec2(self.width / 2.0, self.height),
        };

        let color = opts.color.unwrap_or(self.default_color);

        let outline_size = if !matches!(opts.outline, TextOutline::None) && !self.outline.is_null()
        {
            unsafe {
                TTF_SetTextColorFloat(
                    self.outline,
                    self.default_outline.r,
                    self.default_outline.g,
                    self.default_outline.b,
                    self.default_outline.a * opts.alpha,
                );
                TTF_DrawRendererText(self.outline, pos.0, pos.1);
                let outline_size = TTF_GetFontOutline(TTF_GetTextFont(self.outline));
                match opts.outline {
                    TextOutline::Outline => outline_size * 2,
                    TextOutline::Shadow => outline_size,
                    TextOutline::None => unreachable!(),
                }
            }
        } else {
            0
        } as f32;
        unsafe {
            TTF_SetTextColorFloat(self.text, color.r, color.g, color.b, color.a * opts.alpha);
            TTF_DrawRendererText(self.text, pos.0 + outline_size, pos.1 + outline_size);
        }
    }
}
