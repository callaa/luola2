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

use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ColorDiff {
    pub rd: f32,
    pub gd: f32,
    pub bd: f32,
    pub ad: f32,
}

impl Color {
    pub const PLAYER_COLORS: [Color; 4] = [
        Color::new(0.2, 0.2, 1.0),
        Color::new(1.0, 0.2, 0.0),
        Color::new(0.2, 1.0, 0.2),
        Color::new(1.0, 1.0, 0.4),
    ];

    pub const WHITE: Color = Color::new(1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn new_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_argb_u32(c: u32) -> Self {
        Self {
            r: ((c & 0x00ff0000) >> 16) as f32 / 255.0,
            g: ((c & 0x0000ff00) >> 8) as f32 / 255.0,
            b: (c & 0x000000ff) as f32 / 255.0,
            a: ((c & 0xff000000) >> 24) as f32 / 255.0,
        }
    }
    pub fn with_alpha(&self, a: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    /// Standard player colors (or white if not an active player)
    pub fn player_color(id: i32) -> Color {
        if id > 0 && id <= Self::PLAYER_COLORS.len() as i32 {
            Self::PLAYER_COLORS[id as usize - 1]
        } else {
            Self::WHITE
        }
    }
}

impl Sub for Color {
    type Output = ColorDiff;

    fn sub(self, other: Self) -> Self::Output {
        ColorDiff {
            rd: self.r - other.r,
            gd: self.g - other.g,
            bd: self.b - other.b,
            ad: self.a - other.a,
        }
    }
}

impl Add<ColorDiff> for Color {
    type Output = Self;

    fn add(self, other: ColorDiff) -> Self::Output {
        Self {
            r: (self.r + other.rd).clamp(0.0, 1.0),
            g: (self.g + other.gd).clamp(0.0, 1.0),
            b: (self.b + other.bd).clamp(0.0, 1.0),
            a: (self.a + other.ad).clamp(0.0, 1.0),
        }
    }
}

impl ColorDiff {
    pub fn new() -> Self {
        Self {
            rd: 0.0,
            gd: 0.0,
            bd: 0.0,
            ad: 0.0,
        }
    }
}

impl Div<f32> for ColorDiff {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            rd: self.rd / rhs,
            gd: self.gd / rhs,
            bd: self.bd / rhs,
            ad: self.ad / rhs,
        }
    }
}

impl Mul<f32> for ColorDiff {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            rd: self.rd * rhs,
            gd: self.gd * rhs,
            bd: self.bd * rhs,
            ad: self.ad * rhs,
        }
    }
}

impl Default for ColorDiff {
    fn default() -> Self {
        Self::new()
    }
}
