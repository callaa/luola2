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

use core::cmp::{max, min};
use sdl3_sys::rect::{SDL_FRect, SDL_GetRectIntersectionFloat, SDL_Rect};
use std::fmt;

use crate::math::Vec2;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RectF(pub SDL_FRect);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Rect(pub(super) SDL_Rect);

impl fmt::Display for RectF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {} : {}x{})",
            self.0.x, self.0.y, self.0.w, self.0.h
        )
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({}, {} : {}x{})",
            self.0.x, self.0.y, self.0.w, self.0.h
        )
    }
}

impl RectF {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self(SDL_FRect { x, y, w, h })
    }

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }

    pub fn w(&self) -> f32 {
        self.0.w
    }

    pub fn h(&self) -> f32 {
        self.0.h
    }

    pub fn right(&self) -> f32 {
        self.0.x + self.0.w
    }

    pub fn bottom(&self) -> f32 {
        self.0.y + self.0.h
    }

    pub fn topleft(&self) -> Vec2 {
        Vec2(self.0.x, self.0.y)
    }

    pub fn topright(&self) -> Vec2 {
        Vec2(self.0.x + self.0.w, self.0.y)
    }

    pub fn bottomleft(&self) -> Vec2 {
        Vec2(self.0.x, self.0.y + self.0.h)
    }

    pub fn center(&self) -> Vec2 {
        Vec2(self.0.x + self.0.w / 2.0, self.0.y + self.0.h / 2.0)
    }

    pub fn size(&self) -> (f32, f32) {
        (self.0.w, self.0.h)
    }

    pub fn offset(&self, x: f32, y: f32) -> RectF {
        RectF::new(self.0.x + x, self.0.y + y, self.0.w, self.0.h)
    }

    /// Get the intersection of two rectangles
    pub fn intersection(&self, other: RectF) -> Option<RectF> {
        let mut result = SDL_FRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };

        if unsafe { SDL_GetRectIntersectionFloat(&self.0, &other.0, &mut result) } {
            Some(result.into())
        } else {
            None
        }
    }

    /// Check if the given point is inside this rectangle
    pub fn contains(&self, point: Vec2) -> bool {
        self.0.x <= point.0
            && self.0.x + self.0.w < point.0
            && self.0.y <= point.1
            && self.0.y + self.0.h < point.1
    }
}

impl From<RectF> for SDL_FRect {
    fn from(rect: RectF) -> SDL_FRect {
        rect.0
    }
}

impl From<SDL_FRect> for RectF {
    fn from(rect: SDL_FRect) -> RectF {
        RectF(rect)
    }
}

impl From<Rect> for RectF {
    fn from(rect: Rect) -> Self {
        Self::new(
            rect.x() as f32,
            rect.y() as f32,
            rect.w() as f32,
            rect.h() as f32,
        )
    }
}

impl mlua::UserData for RectF {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x()));
        fields.add_field_method_get("y", |_, this| Ok(this.y()));
        fields.add_field_method_get("w", |_, this| Ok(this.w()));
        fields.add_field_method_get("h", |_, this| Ok(this.h()));
    }
}

impl mlua::FromLua for RectF {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "RectF".to_owned(),
                message: Some("expected RectF".to_string()),
            }),
        }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self(SDL_Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        })
    }
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self(SDL_Rect { x, y, w, h })
    }

    pub fn x(&self) -> i32 {
        self.0.x
    }

    pub fn y(&self) -> i32 {
        self.0.y
    }

    pub fn w(&self) -> i32 {
        self.0.w
    }

    pub fn h(&self) -> i32 {
        self.0.h
    }

    pub fn right(&self) -> i32 {
        self.0.x + self.0.w - 1
    }

    pub fn bottom(&self) -> i32 {
        self.0.y + self.0.h - 1
    }

    pub fn size(&self) -> (i32, i32) {
        (self.0.w, self.0.h)
    }

    pub fn intersected(&self, other: Rect) -> Option<Rect> {
        let leftx = max(self.x(), other.x());
        let rightx = min(self.x() + self.w(), other.x() + other.w());
        let topy = max(self.y(), other.y());
        let btmy = min(self.y() + self.h(), other.y() + other.h());

        if leftx < rightx && topy < btmy {
            Some(Rect::new(leftx, topy, rightx - leftx, btmy - topy))
        } else {
            None
        }
    }

    pub fn offset(&self, x: i32, y: i32) -> Self {
        Self::new(self.0.x + x, self.0.y + y, self.0.w, self.0.h)
    }
}

impl From<Rect> for SDL_Rect {
    fn from(rect: Rect) -> SDL_Rect {
        rect.0
    }
}

impl From<SDL_Rect> for Rect {
    fn from(rect: SDL_Rect) -> Rect {
        Rect(rect)
    }
}
