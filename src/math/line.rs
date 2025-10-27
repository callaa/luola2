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

use sdl3_sys::rect::{SDL_GetRectAndLineIntersection, SDL_GetRectAndLineIntersectionFloat};

use crate::math::Rect;

use super::{RectF, Vec2};
use std::{fmt, ops::Div};

#[derive(Clone, Copy)]
pub struct LineF(pub Vec2, pub Vec2);

#[derive(Clone, Copy)]
pub struct Line {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl fmt::Display for LineF {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}->{}", self.0, self.1)
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{})->({},{})", self.x1, self.y1, self.x2, self.y2)
    }
}

impl LineF {
    pub fn intersected(&self, rect: &RectF) -> Option<Self> {
        let mut i = *self;

        if unsafe {
            SDL_GetRectAndLineIntersectionFloat(
                &rect.0, &mut i.0.0, &mut i.0.1, &mut i.1.0, &mut i.1.1,
            )
        } {
            Some(i)
        } else {
            None
        }
    }
}

impl Line {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn offset(&self, x: i32, y: i32) -> Self {
        Self {
            x1: self.x1 + x,
            y1: self.y1 + y,
            x2: self.x2 + x,
            y2: self.y2 + y,
        }
    }
    pub fn intersected(&self, rect: &Rect) -> Option<Self> {
        let mut i = *self;

        if unsafe {
            SDL_GetRectAndLineIntersection(&rect.0, &mut i.x1, &mut i.y1, &mut i.x2, &mut i.y2)
        } {
            Some(i)
        } else {
            None
        }
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.x1 == other.x1 && self.y1 == other.y1 && self.x2 == other.x2 && self.y2 == other.y2
    }
}

impl Eq for Line {}

impl Div<f32> for LineF {
    type Output = LineF;
    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs, self.1 / rhs)
    }
}

impl From<LineF> for Line {
    fn from(value: LineF) -> Self {
        Self {
            x1: value.0.0 as i32,
            y1: value.0.1 as i32,
            x2: value.1.0 as i32,
            y2: value.1.1 as i32,
        }
    }
}
