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

use anyhow::{Result, anyhow};
use sdl3_sys::blendmode::SDL_BLENDMODE_BLEND;
use sdl3_sys::rect::SDL_FPoint;
use sdl3_sys::render::{
    SDL_DestroyRenderer, SDL_RenderDebugText, SDL_RenderFillRect, SDL_RenderPoint,
    SDL_RenderPoints, SDL_SetRenderDrawBlendMode, SDL_SetRenderDrawColorFloat,
};
use sdl3_sys::video::{SDL_SetWindowFullscreen, SDL_WINDOW_FULLSCREEN, SDL_WINDOW_RESIZABLE};
use sdl3_ttf_sys::ttf::{
    TTF_CreateRendererTextEngine, TTF_DestroyRendererTextEngine, TTF_Init, TTF_TextEngine,
};
use std::ffi::CStr;
use std::path::Path;
use std::ptr::{null, null_mut};

use crate::gfx::FontSet;
use crate::math::{Rect, RectF, Vec2};

use super::texturestore::*;
use super::{Color, SdlError, SdlResult};
use sdl3_sys::{
    pixels::SDL_ALPHA_OPAQUE,
    rect::SDL_Rect,
    render::{
        SDL_CreateWindowAndRenderer, SDL_GetRenderViewport, SDL_RenderClear, SDL_RenderLine,
        SDL_RenderPresent, SDL_Renderer, SDL_SetRenderDrawColor, SDL_SetRenderVSync,
        SDL_SetRenderViewport,
    },
    video::SDL_Window,
};

pub struct Renderer {
    window: *mut SDL_Window,
    pub(super) renderer: *mut SDL_Renderer,
    texturestore: TextureStore,
    fontset: Option<FontSet>,
    pub(super) textengine: *mut TTF_TextEngine,
    width: i32,
    height: i32,
    fullscreen: bool,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            TTF_DestroyRendererTextEngine(self.textengine);
            SDL_DestroyRenderer(self.renderer);
        }
    }
}
impl Renderer {
    pub fn create(fullscreen: bool) -> SdlResult<Self> {
        let mut window: *mut SDL_Window = null_mut();
        let mut renderer: *mut SDL_Renderer = null_mut();

        let mut flags = SDL_WINDOW_RESIZABLE;
        if fullscreen {
            flags |= SDL_WINDOW_FULLSCREEN;
        }

        if !unsafe {
            SDL_CreateWindowAndRenderer(
                c"Luola II".as_ptr(),
                1024,
                768,
                flags,
                &mut window,
                &mut renderer,
            )
        } {
            return Err(SdlError::get_error("Couldn't create renderer"));
        }

        if !unsafe { SDL_SetRenderVSync(renderer, 1) } {
            return Err(SdlError::get_error("Couldn't enable V-Sync"));
        }

        if !unsafe { TTF_Init() } {
            return Err(SdlError::get_error("Couldn't init SDL TTF"));
        }

        unsafe {
            SDL_SetRenderDrawBlendMode(renderer, SDL_BLENDMODE_BLEND);
        }

        let textengine = unsafe { TTF_CreateRendererTextEngine(renderer) };
        if textengine.is_null() {
            return Err(SdlError::get_error("Couldn't create text engine"));
        }

        Ok(Self {
            window,
            renderer,
            texturestore: TextureStore::new(),
            fontset: None,
            textengine,
            width: 1024,
            height: 768,
            fullscreen,
        })
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        unsafe {
            SDL_SetWindowFullscreen(self.window, self.fullscreen);
        }
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn load_textures(&mut self, texture_config: &Path) -> Result<()> {
        if self.texture_store().count() > 0 {
            return Err(anyhow!("Textures already loaded"));
        }

        self.texturestore = TextureStore::load_from_toml(self, texture_config)?;
        Ok(())
    }

    pub fn load_fontset(&mut self, fontset_config: &Path) -> Result<()> {
        if self.fontset.is_some() {
            return Err(anyhow!("Fontset already loaded"));
        }

        self.fontset = Some(FontSet::load_from_toml(fontset_config)?);
        Ok(())
    }

    pub fn texture_store(&self) -> &TextureStore {
        &self.texturestore
    }

    pub fn try_fontset(&self) -> Result<&FontSet> {
        self.fontset.as_ref().ok_or(anyhow!("fontset not loaded"))
    }

    pub fn fontset(&self) -> &FontSet {
        if let Some(fs) = self.fontset.as_ref() {
            fs
        } else {
            panic!("fontset not loaded!")
        }
    }

    pub fn set_viewport(&mut self, rect: Rect) -> SdlResult<()> {
        if !unsafe { SDL_SetRenderViewport(self.renderer, &rect.into()) } {
            return Err(SdlError::get_error("couldn't set viewport"));
        }

        self.width = rect.w();
        self.height = rect.h();

        Ok(())
    }

    pub fn reset_viewport(&mut self) -> SdlResult<()> {
        if !unsafe { SDL_SetRenderViewport(self.renderer, null()) } {
            return Err(SdlError::get_error("couldn't set viewport"));
        }

        let mut rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };

        if !unsafe { SDL_GetRenderViewport(self.renderer, &mut rect) } {
            return Err(SdlError::get_error("couldn't set viewport"));
        }

        self.width = rect.w;
        self.height = rect.h;

        Ok(())
    }

    pub fn clear(&self) {
        unsafe {
            SDL_SetRenderDrawColor(self.renderer, 0, 0, 0, SDL_ALPHA_OPAQUE);
            SDL_RenderClear(self.renderer);
        }
    }

    pub fn draw_debug_text(&self, text: &CStr, x: f32, y: f32) {
        unsafe {
            SDL_SetRenderDrawColor(self.renderer, 255, 0, 0, SDL_ALPHA_OPAQUE);
            SDL_RenderDebugText(self.renderer, x, y, text.as_ptr());
        }
    }

    pub fn draw_filled_rectangle(&self, rect: RectF, color: &Color) {
        unsafe {
            SDL_SetRenderDrawColorFloat(self.renderer, color.r, color.g, color.b, color.a);
            SDL_RenderFillRect(self.renderer, &rect.into());
        }
    }

    pub fn draw_point(&self, point: Vec2, color: &Color) {
        unsafe {
            SDL_SetRenderDrawColorFloat(self.renderer, color.r, color.g, color.b, color.a);
            SDL_RenderPoint(self.renderer, point.0, point.1);
        }
    }

    pub fn draw_points(&self, points: &[SDL_FPoint], color: &Color) {
        unsafe {
            SDL_SetRenderDrawColorFloat(self.renderer, color.r, color.g, color.b, color.a);
            SDL_RenderPoints(self.renderer, points.as_ptr(), points.len() as i32);
        }
    }

    pub fn draw_line_segments_iter<I>(&self, color: Color, lines: I)
    where
        I: Iterator<Item = (f32, f32, f32, f32)>,
    {
        unsafe {
            SDL_SetRenderDrawColorFloat(self.renderer, color.r, color.g, color.b, color.a);
        }

        lines.for_each(|l| unsafe {
            SDL_RenderLine(self.renderer, l.0, l.1, l.2, l.3);
        });
    }

    /**
     * Draw a debugging grid
     */
    pub fn draw_debug_grid(&self, offset: Vec2, size: f32) {
        let xoff = offset.0 % size;
        let yoff = offset.1 % size;

        unsafe {
            SDL_SetRenderDrawColor(self.renderer, 128, 0, 0, SDL_ALPHA_OPAQUE);
        }

        let mut x = -xoff;
        while x < self.width as f32 {
            unsafe {
                SDL_RenderLine(self.renderer, x, 0.0, x, self.height as f32);
            }
            x += size;
        }

        let mut y = -yoff;
        while y < self.height as f32 {
            unsafe {
                SDL_RenderLine(self.renderer, 0.0, y, self.width as f32, y);
            }
            y += size;
        }
    }

    pub fn present(&self) {
        unsafe {
            SDL_RenderPresent(self.renderer);
        }
    }
}
