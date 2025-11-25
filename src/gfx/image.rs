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

use anyhow::Result;
use core::slice;
use sdl3_image_sys::image::IMG_Load;
use sdl3_sys::{
    pixels::{SDL_PIXELFORMAT_ARGB8888, SDL_PIXELFORMAT_INDEX8, SDL_Palette},
    rect::SDL_Rect,
    surface::{
        SDL_BlitSurface, SDL_ConvertSurface, SDL_DestroySurface, SDL_GetSurfacePalette,
        SDL_SCALEMODE_LINEAR, SDL_SCALEMODE_NEAREST, SDL_ScaleSurface, SDL_Surface,
    },
};
use std::path::PathBuf;

use super::{SdlError, SdlResult};
use crate::{fs::pathbuf_to_cstring, math::Rect};

pub struct Image(pub(super) *mut SDL_Surface);

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            SDL_DestroySurface(self.0);
        }
    }
}

impl Image {
    pub fn from_file(path: PathBuf) -> Result<Image> {
        let path = pathbuf_to_cstring(path)?;
        let surface = unsafe { IMG_Load(path.as_ptr()) };
        if surface.is_null() {
            return Err(SdlError::get_error("IMG_load").into());
        }

        Ok(Image(surface))
    }

    pub fn width(&self) -> i32 {
        unsafe { (*self.0).w }
    }

    pub fn height(&self) -> i32 {
        unsafe { (*self.0).h }
    }

    pub fn ensure_argb888(self) -> SdlResult<Image> {
        let format = SDL_PIXELFORMAT_ARGB8888;

        if unsafe { &*self.0 }.format == format {
            return Ok(self);
        }

        let newsurface = unsafe { SDL_ConvertSurface(self.0, format) };

        if newsurface.is_null() {
            return Err(SdlError::get_error("Couldn't convert surface"));
        }

        Ok(Image(newsurface))
    }

    pub fn argb8888_pixels(&self) -> Option<&[u32]> {
        let surface = unsafe { &*self.0 };
        if surface.format != SDL_PIXELFORMAT_ARGB8888 {
            return None;
        }

        Some(unsafe {
            slice::from_raw_parts(
                surface.pixels as *const u32,
                (surface.w * surface.h) as usize,
            )
        })
    }

    pub fn argb8888_pixels_mut(&mut self) -> Option<&mut [u32]> {
        let surface = unsafe { &*self.0 };
        if surface.format != SDL_PIXELFORMAT_ARGB8888 {
            return None;
        }

        Some(unsafe {
            slice::from_raw_parts_mut(surface.pixels as *mut u32, (surface.w * surface.h) as usize)
        })
    }

    pub fn indexed_pixels(&self) -> Option<&[u8]> {
        let surface = unsafe { &*self.0 };
        if surface.format != SDL_PIXELFORMAT_INDEX8 {
            return None;
        }

        Some(unsafe {
            slice::from_raw_parts(
                surface.pixels as *const u8,
                (surface.w * surface.h) as usize,
            )
        })
    }

    pub fn palette(&self) -> Option<&SDL_Palette> {
        unsafe { SDL_GetSurfacePalette(self.0).as_ref() }
    }

    pub fn blit(&self, source: Rect, target: &mut Image, dest: (i32, i32)) {
        unsafe {
            SDL_BlitSurface(
                self.0,
                &source.into(),
                target.0,
                &SDL_Rect {
                    x: dest.0,
                    y: dest.1,
                    w: 0,
                    h: 0,
                },
            );
        }
    }

    pub fn scaled(&self, width: i32, height: i32, smooth: bool) -> Result<Image> {
        let scalef = if width > self.width() {
            height as f32 / self.height() as f32
        } else {
            width as f32 / self.width() as f32
        };

        let real_w = (self.width() as f32 * scalef) as i32;
        let real_h = (self.height() as f32 * scalef) as i32;

        let surface = unsafe {
            SDL_ScaleSurface(
                self.0,
                real_w,
                real_h,
                if smooth {
                    SDL_SCALEMODE_LINEAR
                } else {
                    SDL_SCALEMODE_NEAREST
                },
            )
        };

        if surface.is_null() {
            return Err(SdlError::get_error("IMG_load").into());
        }

        Ok(Image(surface))
    }
}
