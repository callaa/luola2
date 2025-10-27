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
    gfx::{Color, Renderer},
    math::Vec2,
};
use fastrand;
use sdl3_sys::rect::SDL_FPoint;

/**
 * Starfield background.
 *
 * The starfield is adapted to viewport size.
 */
pub struct Starfield {
    original_stars: Vec<Vec2>,
    viewport_stars: Vec<SDL_FPoint>,
}

impl Starfield {
    const COUNT: usize = 40;

    pub fn new() -> Self {
        let mut stars = Vec::with_capacity(Self::COUNT);
        for _ in 0..Self::COUNT {
            stars.push(Vec2(fastrand::f32(), fastrand::f32()));
        }

        Self {
            original_stars: stars,
            viewport_stars: Vec::with_capacity(Self::COUNT),
        }
    }

    pub fn recalculate(&mut self, width: f32, height: f32) {
        self.viewport_stars.clear();
        for star in &self.original_stars {
            self.viewport_stars.push(SDL_FPoint {
                x: star.0 * width,
                y: star.1 * height,
            });
        }
    }

    pub fn render(&self, renderer: &Renderer) {
        renderer.draw_points(&self.viewport_stars, &Color::WHITE);
    }
}
