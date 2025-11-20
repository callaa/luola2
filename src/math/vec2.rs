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
    f32::consts::PI,
    fmt,
    ops::{Add, Div, Mul, Sub},
};

use mlua::{self, MetaMethod};

#[derive(Debug, Copy, Clone)]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    pub const ZERO: Vec2 = Vec2(0.0, 0.0);

    pub fn for_angle(a: f32, mag: f32) -> Self {
        let arad = a * PI / 180.0;
        Vec2(arad.cos() * mag, arad.sin() * mag)
    }

    pub fn angle(&self) -> f32 {
        f32::atan2(-self.1, self.0) * (180.0 / PI)
    }

    pub fn dist_squared(self, other: Self) -> f32 {
        (self.0 - other.0).powf(2.0) + (self.1 - other.1).powf(2.0)
    }

    pub fn dist(self, other: Self) -> f32 {
        self.dist_squared(other).sqrt()
    }

    pub fn manhattan_dist(self, other: Self) -> f32 {
        (self.0 - other.0).abs() + (self.1 - other.1).abs()
    }

    pub fn magnitude_squared(self) -> f32 {
        self.0.powf(2.0) + self.1.powf(2.0)
    }

    pub fn magnitude(self) -> f32 {
        self.magnitude_squared().sqrt()
    }

    pub fn normalized(self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            Self(0.0, 0.0)
        } else {
            Self(self.0 / mag, self.1 / mag)
        }
    }

    pub fn dot(self, other: Self) -> f32 {
        self.0 * other.0 + self.1 * other.1
    }

    pub fn project(self, other: Self) -> Self {
        let k = self.dot(other) / other.dot(other);
        Vec2(k * other.0, k * other.1)
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Vec2(0.0, 0.0)
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Vec2(self.0 + other.0, self.1 + other.1)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Vec2(self.0 - other.0, self.1 - other.1)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Vec2(self.0 * rhs, self.1 * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self {
        Vec2(self.0 / rhs, self.1 / rhs)
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        self.dist_squared(*other) < 0.001
    }
}

impl mlua::UserData for Vec2 {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0));
        fields.add_field_method_get("y", |_, this| Ok(this.1));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("normalized", |_, this, _: ()| Ok(this.normalized()));
        methods.add_method("dist", |_, this, other: Vec2| Ok(this.dist(other)));
        methods.add_method("dist_squared", |_, this, other: Vec2| {
            Ok(this.dist_squared(other))
        });
        methods.add_method("angle", |_, this, _: ()| Ok(this.angle()));
        methods.add_method("magnitude", |_, this, _: ()| Ok(this.magnitude()));

        methods.add_meta_function(MetaMethod::Add, |_, (v1, v2): (Vec2, Vec2)| Ok(v1 + v2));
        methods.add_meta_function(MetaMethod::Sub, |_, (v1, v2): (Vec2, Vec2)| Ok(v1 - v2));
        methods.add_meta_function(MetaMethod::Mul, |_, (v1, f): (Vec2, f32)| Ok(v1 * f));
        methods.add_meta_function(MetaMethod::Div, |_, (v1, f): (Vec2, f32)| Ok(v1 / f));
        methods.add_meta_function(MetaMethod::ToString, |_, v: Vec2| Ok(v.to_string()));
    }
}

impl mlua::FromLua for Vec2 {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(t) => Ok(Vec2(t.get(1)?, t.get(2)?)),
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vec2".to_owned(),
                message: Some("expected Vec2".to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_vec2_ops() {
        let v1 = Vec2(1.0, 2.0);
        let v2 = Vec2(3.0, 4.0);

        assert_eq!(v1 + v2, Vec2(4.0, 6.0));

        assert_eq!(v1 * 3.0, Vec2(3.0, 6.0));

        assert!((v1.magnitude() - 2.236).abs() < 0.0001);

        assert_eq!(v1.normalized(), Vec2(0.447, 0.894));
    }

    #[test]
    fn test_trig_ops() {
        assert_eq!(Vec2::for_angle(0.0, 1.0), Vec2(1.0, 0.0));
        assert_eq!(Vec2::for_angle(90.0, 2.0), Vec2(0.0, 2.0));
        assert_eq!(Vec2::for_angle(180.0, 3.0), Vec2(-3.0, 0.0));
    }
}
