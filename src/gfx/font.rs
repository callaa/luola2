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
    TTF_GetTextColorFloat, TTF_GetTextSize, TTF_OpenFont, TTF_SetTextColorFloat, TTF_SetTextString,
    TTF_SetTextWrapWidth, TTF_Text,
};

use crate::{
    fs::pathbuf_to_cstring,
    gfx::{Color, Renderer, SdlError},
    math::Vec2,
};

pub struct Font {
    font: *mut TTF_Font,
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
    width: f32,
    height: f32,
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe {
            TTF_DestroyText(self.text);
        }
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        let font = unsafe { TTF_CopyFont(self.font) };
        if font.is_null() {
            panic!("Font copy failed!");
        }
        Self { font }
    }
}

impl Font {
    pub fn from_file(path: PathBuf, ptsize: f32) -> Result<Font> {
        let path = pathbuf_to_cstring(path)?;
        let font = unsafe { TTF_OpenFont(path.as_ptr(), ptsize) };
        if font.is_null() {
            return Err(SdlError::get_error("Couldn't open font").into());
        }

        Ok(Self { font })
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

        let mut width: c_int = 0;
        let mut height: c_int = 0;

        unsafe {
            TTF_GetTextSize(text, &mut width, &mut height);
        }

        Ok(Text {
            text,
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

    pub fn with_wrapwidth(self, width: i32) -> Self {
        unsafe {
            TTF_SetTextWrapWidth(self.text, width);
        }
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.set_color(color);
        self
    }

    pub fn set_text(&mut self, text: &str) {
        unsafe {
            TTF_SetTextString(self.text, text.as_ptr() as *const i8, text.len());
        }
    }
    pub fn set_color(&mut self, color: Color) {
        let mut width: c_int = 0;
        let mut height: c_int = 0;
        unsafe {
            TTF_SetTextColorFloat(self.text, color.r, color.g, color.b, color.a);
            TTF_GetTextSize(self.text, &mut width, &mut height);
        }

        self.width = width as f32;
        self.height = height as f32;
    }

    pub fn set_alpha(&mut self, a: f32) {
        let mut r: f32 = 0.0;
        let mut g: f32 = 0.0;
        let mut b: f32 = 0.0;

        unsafe {
            TTF_GetTextColorFloat(self.text, &mut r, &mut g, &mut b, null_mut());
        }

        self.set_color(Color::new_rgba(r, g, b, a));
    }

    pub fn render_hcenter(&self, w: f32, y: f32) {
        self.render(Vec2((w - self.width) / 2.0, y));
    }
    pub fn render(&self, pos: Vec2) {
        unsafe {
            TTF_DrawRendererText(self.text, pos.0, pos.1);
        }
    }
}
