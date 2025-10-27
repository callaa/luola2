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

use crate::gfx::{Color, Renderer};

/**
 * An animated starfield background for menus
 */
#[derive(Clone)]

pub struct AnimatedStarfield {
    stars: Vec<Star>,
    width: f32,
    height: f32,
}

#[derive(Clone)]
struct Star {
    target_x: f32,
    target_y: f32,
    z: f32,
    start_x: f32,
    start_y: f32,
    px: f32,
    py: f32,
    px2: f32,
    py2: f32,
}

impl AnimatedStarfield {
    pub fn new(count: usize, width: f32, height: f32) -> Self {
        let mut stars = Vec::with_capacity(count);

        for _ in 0..count {
            stars.push(Self::make_star(
                fastrand::f32() * width,
                fastrand::f32() * height,
                50.0 + fastrand::f32() * 50.0,
                width,
                height,
                10.0,
            ));
        }

        Self {
            stars,
            width,
            height,
        }
    }

    pub fn update_screensize(&mut self, size: (i32, i32)) {
        self.width = size.0 as f32;
        self.height = size.1 as f32;

        for s in self.stars.iter_mut() {
            *s = Self::make_star(
                fastrand::f32() * self.width,
                fastrand::f32() * self.height,
                50.0 + fastrand::f32() * 50.0,
                self.width,
                self.height,
                10.0,
            );
        }
    }

    pub fn step(&mut self, timestep: f32) {
        for (i, star) in self.stars.iter_mut().enumerate() {
            star.z -= timestep * 20.0;
            if star.z <= 0.0 {
                *star = Self::make_star(
                    self.width / 2.0,
                    self.height / 2.0,
                    i as f32 * 2.0 + 1.0,
                    self.width,
                    self.height,
                    5000.0,
                );
            }

            star.px = star.px2;
            star.py = star.py2;
            star.px2 = star.target_x / star.z + star.start_x;
            star.py2 = star.target_y / star.z + star.start_y;
        }
    }

    pub fn render(&self, renderer: &Renderer) {
        renderer.draw_line_segments_iter(
            Color::WHITE,
            self.stars.iter().map(|s| (s.px, s.py, s.px2, s.py2)),
        );
    }

    fn make_star(start_x: f32, start_y: f32, z: f32, width: f32, height: f32, scale: f32) -> Star {
        let target_x = ((start_x - width / 2.0) + fastrand::f32() * 20.0 - 10.0) * scale;
        let target_y = ((start_y - height / 2.0) + fastrand::f32() * 20.0 - 10.0) * scale;
        let px = target_x / z + start_x;
        let py = target_y / z + start_y;
        Star {
            target_x,
            target_y,
            z,
            start_x,
            start_y,
            px,
            py,
            px2: px,
            py2: py,
        }
    }
}
