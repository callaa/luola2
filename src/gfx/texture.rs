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

use std::{
    collections::HashMap,
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::null,
};

use crate::{
    fs::pathbuf_to_cstring,
    gfx::{Color, Image, TexAlt},
    math::{RectF, Vec2},
};

use super::{Renderer, SdlError};
use anyhow::Result;
use sdl3_image_sys::image::IMG_LoadTexture;
use sdl3_sys::{
    blendmode::SDL_BLENDMODE_BLEND,
    pixels::SDL_PIXELFORMAT_ARGB8888,
    rect::SDL_Rect,
    render::{
        SDL_CreateTexture, SDL_CreateTextureFromSurface, SDL_DestroyTexture, SDL_GetTextureSize,
        SDL_RenderTexture, SDL_RenderTextureRotated, SDL_RenderTextureTiled,
        SDL_SetTextureAlphaModFloat, SDL_SetTextureBlendMode, SDL_SetTextureColorModFloat,
        SDL_SetTextureScaleMode, SDL_Texture, SDL_TextureAccess, SDL_UpdateTexture,
    },
    surface::{SDL_FLIP_HORIZONTAL, SDL_FLIP_NONE, SDL_SCALEMODE_LINEAR, SDL_SCALEMODE_NEAREST},
};
use serde::Deserialize;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TextureConfig {
    #[serde(rename = "file")]
    filename: String,
    subrect: Option<(i32, i32, i32, i32)>, // use only a portion of the texture. (texture atlas)
    #[serde(default)]
    frames: i32,      // If greater than one, this is an animated texture
    #[serde(default)]
    angles: i32,      // number of angle sprites
    duration: Option<f32>,                 // animation duration in seconds (default=length/60)
    width: Option<i32>,                    // Width of a single frame/subtexture
    height: Option<i32>,                   // Height of a single frame/subtexture

    scale: Option<TextureScaleMode>,
    #[serde(default)]
    needs_rotation: bool, // Hint to the renderer that this sprite should be rotated in the direction of the motion
}

#[derive(serde::Deserialize, Debug)]
pub struct TextureAltConfig {
    #[serde(rename = "file")]
    filename: String,
    subrect: Option<(i32, i32, i32, i32)>,
}

#[derive(serde::Deserialize, Debug)]
pub struct TextureConfigWithAlts {
    #[serde(flatten)]
    pub main: TextureConfig,
    pub alts: Option<HashMap<TexAlt, TextureAltConfig>>,
}

pub struct Texture {
    tex: *mut SDL_Texture,
    width: f32,
    height: f32,
    subrect: RectF,
    pub(super) angles: i32,
    pub(super) frames: i32,
    pub(super) frame_duration: f32,
    needs_rotation: bool,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum TextureScaleMode {
    Nearest,
    Linear,
}

#[derive(Clone, Copy)]
pub enum RenderMode {
    Normal,
    Rotated(f32, bool), // angle, mirror
    Tiled(f32),         // scale
}

#[derive(Clone, Copy)]
pub enum RenderDest {
    Fill,
    Rect(RectF),
    /// Fit inside this rectangle, maintaining aspect ratio
    FitIn(RectF),
    Centered(Vec2),
}

#[derive(Clone)]
pub struct RenderOptions {
    pub source: Option<RectF>,
    pub dest: RenderDest,
    pub mode: RenderMode,
    pub color: Color,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            source: None,
            dest: RenderDest::Fill,
            mode: RenderMode::Normal,
            color: Color::WHITE,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { SDL_DestroyTexture(self.tex) };
    }
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        unsafe { self.tex.as_mut() }.unwrap().refcount += 1;
        Self {
            tex: self.tex,
            width: self.width,
            height: self.height,
            subrect: self.subrect,
            angles: self.angles,
            frames: self.frames,
            frame_duration: self.frame_duration,
            needs_rotation: self.needs_rotation,
        }
    }
}

impl Texture {
    pub fn from_config(
        renderer: &Renderer,
        root: &Path,
        config: &TextureConfig,
        alt_config: Option<&TextureAltConfig>,
        shared_textures: &mut HashMap<String, *mut SDL_Texture>,
    ) -> Result<Texture> {
        let filename = alt_config.map_or_else(|| &config.filename, |a| &a.filename);

        let mut tex = if shared_textures.contains_key(filename) {
            let tex = Self::from_texture(shared_textures[filename]);
            if let Ok(t) = &tex {
                unsafe { t.tex.as_mut().unwrap() }.refcount += 1;
            }
            tex
        } else {
            let tex = Self::from_file(renderer, [root, Path::new(filename)].iter().collect());
            if let Ok(t) = &tex {
                shared_textures.insert(filename.clone(), t.tex);
            }
            tex
        }?;

        let subrect = alt_config.map_or_else(|| config.subrect, |a| a.subrect);
        if let Some(sr) = subrect {
            tex.subrect = RectF::new(sr.0 as f32, sr.1 as f32, sr.2 as f32, sr.3 as f32);
            tex.width = tex.subrect.w();
            tex.height = tex.subrect.h();
        }

        tex.frames = config.frames.max(1);
        tex.angles = config.angles.max(1);

        if let Some(duration) = config.duration {
            tex.frame_duration = duration / tex.frames as f32;
        }

        if let Some(width) = config.width {
            tex.width = width as f32;
        }

        if let Some(height) = config.height {
            tex.height = height as f32;
        }

        tex.needs_rotation = config.needs_rotation;

        if let Some(scale) = config.scale.as_ref() {
            tex.set_scalemode(*scale);
        }

        unsafe {
            SDL_SetTextureBlendMode(tex.tex, SDL_BLENDMODE_BLEND);
        }
        Ok(tex)
    }

    pub fn from_file(renderer: &Renderer, path: PathBuf) -> Result<Texture> {
        let path = pathbuf_to_cstring(path)?;
        let tex = unsafe { IMG_LoadTexture(renderer.renderer, path.as_ptr()) };

        Self::from_texture(tex)
    }

    pub fn from_image(renderer: &Renderer, image: &Image) -> Result<Texture> {
        let tex = unsafe { SDL_CreateTextureFromSurface(renderer.renderer, image.0) };
        Self::from_texture(tex)
    }

    /// Create a blank streaming texture
    pub fn new_streaming(renderer: &Renderer, width: i32, height: i32) -> Result<Texture> {
        let tex = unsafe {
            SDL_CreateTexture(
                renderer.renderer,
                SDL_PIXELFORMAT_ARGB8888,
                SDL_TextureAccess::STREAMING,
                width,
                height,
            )
        };

        Self::from_texture(tex)
    }

    fn from_texture(tex: *mut SDL_Texture) -> Result<Texture> {
        if tex.is_null() {
            return Err(SdlError::get_error("Couldn't convert image into texture").into());
        }

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        if !unsafe { SDL_GetTextureSize(tex, &mut width, &mut height) } {
            return Err(SdlError::get_error("Couldn't get texture size").into());
        }

        Ok(Texture {
            tex,
            subrect: RectF::new(0.0, 0.0, width, height),
            width,
            height,
            angles: 0,
            frames: 0,
            frame_duration: 1.0,
            needs_rotation: false,
        })
    }

    pub fn clone_subrect(&self, subrect: RectF) -> Self {
        let mut t = self.clone();
        t.subrect = subrect;
        t.width = subrect.w();
        t.height = subrect.h();
        t
    }

    pub fn set_scalemode(&mut self, mode: TextureScaleMode) {
        let mode = match mode {
            TextureScaleMode::Linear => SDL_SCALEMODE_LINEAR,
            TextureScaleMode::Nearest => SDL_SCALEMODE_NEAREST,
        };

        unsafe {
            SDL_SetTextureScaleMode(self.tex, mode);
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    /**
     * This texture is not symmetrical and should be rotated in the direction
     * of the motion.
     *
     * This is just a hint, it's up to the renderer to actually do something
     * about it, if anything.
     * This is typically used for non-round projectile sprites.
     */
    pub fn needs_rotation(&self) -> bool {
        self.needs_rotation
    }

    pub fn subframe_rect(&self, frame: i32) -> RectF {
        debug_assert!(frame >= 0 && frame < (self.frames * self.angles));
        let row = (self.subrect.w() / self.width) as i32;

        RectF::new(
            (frame % row) as f32 * self.width,
            (frame / row) as f32 * self.height,
            self.width,
            self.height,
        )
    }

    /// Get a texture sized rectangle centered on the given coordinates
    fn centered_rect(&self, pos: Vec2) -> RectF {
        RectF::new(
            pos.0 - self.width / 2.0,
            pos.1 - self.height / 2.0,
            self.width,
            self.height,
        )
    }

    /// Write new pixels into this texture
    pub fn write_pixels(&mut self, source: &[u32], x: i32, y: i32, w: i32, h: i32) {
        assert_eq!(source.len(), (w * h) as usize);
        if !unsafe {
            SDL_UpdateTexture(
                self.tex,
                &SDL_Rect { x, y, w, h },
                source.as_ptr() as *const c_void,
                w * 4,
            )
        } {
            SdlError::log("SDL_UpdateTexture call failed");
        }
    }

    pub fn render(&self, renderer: &Renderer, options: &RenderOptions) {
        let dest = match options.dest {
            RenderDest::Fill => None,
            RenderDest::Centered(pos) => Some(self.centered_rect(pos)),
            RenderDest::FitIn(rect) => {
                let scale = if rect.w() < self.width || rect.h() < self.height {
                    (rect.w() / self.width).min(rect.h() / self.height)
                } else {
                    1.0
                };
                let w = self.width * scale;
                let h = self.height * scale;
                Some(RectF::new(
                    rect.x() + (rect.w() - w) / 2.0,
                    rect.y() + (rect.h() - h) / 2.0,
                    w,
                    h,
                ))
            }
            RenderDest::Rect(rect) => Some(rect),
        };

        unsafe {
            SDL_SetTextureColorModFloat(
                self.tex,
                options.color.r,
                options.color.g,
                options.color.b,
            );
            SDL_SetTextureAlphaModFloat(self.tex, options.color.a);
        }

        let result = match options.mode {
            RenderMode::Normal => {
                let source = if let Some(s) = options.source {
                    RectF::new(
                        self.subrect.x() + s.x(),
                        self.subrect.y() + s.y(),
                        s.w(),
                        s.h(),
                    )
                } else {
                    self.subrect
                };

                unsafe {
                    SDL_RenderTexture(
                        renderer.renderer,
                        self.tex,
                        &source.0,
                        match dest {
                            Some(ref r) => &r.0,
                            None => null(),
                        },
                    )
                }
            }
            RenderMode::Rotated(angle, mirror) => {
                let (source, angle) = if self.angles > 1 {
                    let angle = angle.rem_euclid(360.0);
                    let f = (angle * (self.angles - 1) as f32 / 360.0).round();
                    let fract = angle - f * (360.0 / self.angles as f32);
                    (
                        self.subframe_rect(f as i32)
                            .offset(self.subrect.x(), self.subrect.y()),
                        fract,
                    )
                } else {
                    (self.subrect, angle)
                };

                unsafe {
                    SDL_RenderTextureRotated(
                        renderer.renderer,
                        self.tex,
                        &source.0,
                        match dest {
                            Some(ref r) => &r.0,
                            None => null(),
                        },
                        -angle as f64,
                        null(),
                        if mirror {
                            SDL_FLIP_HORIZONTAL
                        } else {
                            SDL_FLIP_NONE
                        },
                    )
                }
            }
            RenderMode::Tiled(scale) => {
                let source = if let Some(s) = options.source {
                    RectF::new(
                        self.subrect.x() + s.x(),
                        self.subrect.y() + s.y(),
                        s.w(),
                        s.h(),
                    )
                } else {
                    self.subrect
                };

                unsafe {
                    SDL_RenderTextureTiled(
                        renderer.renderer,
                        self.tex,
                        &source.0,
                        scale,
                        match dest {
                            Some(ref r) => &r.0,
                            None => null(),
                        },
                    )
                }
            }
        };
        if !result {
            SdlError::log("Texture render");
        }
    }

    /**
     * Render this texture.
     */
    pub fn render_simple(&self, renderer: &Renderer, source: Option<RectF>, dest: Option<RectF>) {
        let mut sr = self.subrect;
        if let Some(s) = source {
            sr = RectF::new(sr.x() + s.x(), sr.y() + s.y(), s.w(), s.h());
        }

        unsafe {
            SDL_RenderTexture(
                renderer.renderer,
                self.tex,
                &sr.0,
                match dest {
                    Some(ref r) => &r.0,
                    None => null(),
                },
            );
        }
    }
}
