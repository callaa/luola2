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

use crate::gfx::{RenderOptions, TexAlt};

use super::{Renderer, TextureId};

/**
 * State wrapper for texture animations
 */
#[derive(Clone, Debug)]
pub struct AnimatedTexture {
    tex: TextureId,
    alt: TexAlt,
    frame: i32,
    frames: i32,
    frame_duration: f32,
    time_remaining: f32,
}

impl AnimatedTexture {
    pub fn new(tex: TextureId) -> Self {
        Self {
            tex,
            alt: TexAlt::Main,
            frame: 0,
            frames: tex.frames(),
            frame_duration: tex.frame_duration(),
            time_remaining: tex.frame_duration(),
        }
    }

    pub fn id(&self) -> TextureId {
        self.tex
    }

    pub fn change_alt(&mut self, alt: TexAlt) {
        self.alt = alt;
    }

    /// Progress animation. Returns true when animation wraps
    pub fn step(&mut self, timestep: f32) -> bool {
        if self.frame_duration > 0.0 {
            self.time_remaining -= timestep;
            if self.time_remaining <= 0.0 {
                self.time_remaining = self.frame_duration;
                self.frame += 1;
                if self.frame >= self.frames {
                    self.frame = 0;
                    return true;
                }
            }
        }
        false
    }

    pub fn render(&self, renderer: &Renderer, options: &RenderOptions) {
        let tex = renderer
            .texture_store()
            .get_texture_alt_fallback(self.tex, self.alt);

        tex.render(
            renderer,
            &RenderOptions {
                source: Some(tex.subframe_rect(self.frame)),
                ..*options
            },
        );
    }
}
