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

use core::ops::Deref;
use std::collections::HashMap;

use mlua;

#[derive(Clone, Copy, Debug)]
pub enum DynamicTerrainCell {
    /// Expanding foam. Expansion starts when counter reaches zero
    Foam {
        limit: i32,
    },

    /// Nanites that eat up destructable terrain. Expansion starts when counter reaches zero
    GreyGoo {
        counter: i32,
        limit: i32,
    },

    // Freezes water and creates a thin layer of frost on solid ground
    Freezer {
        limit: i32,
    },

    // Soaks ground in nitroglycerin and makes it explosive
    Nitro {
        counter: i32,
        limit: i32,
    },

    // Fire spreads and consumes combustible terrain
    // Cinder type terrain is turned into grey colored regular ground
    Fire {
        counter: i32,
        cinder: bool,
    },
}

impl DynamicTerrainCell {
    pub fn destroys_ground(&self) -> bool {
        matches!(
            self,
            DynamicTerrainCell::GreyGoo {
                counter: _,
                limit: _
            } | DynamicTerrainCell::Nitro {
                counter: _,
                limit: _,
            } | DynamicTerrainCell::Fire {
                counter: _,
                cinder: _,
            }
        )
    }

    pub fn from_lua_table(table: &mlua::Table) -> mlua::Result<Self> {
        let typ = table.get::<mlua::String>("type")?;
        match typ.as_bytes().deref() {
            b"Foam" => Ok(DynamicTerrainCell::Foam {
                limit: table.get::<Option<i32>>("limit")?.unwrap_or(20),
            }),
            b"GreyGoo" => Ok(DynamicTerrainCell::GreyGoo {
                counter: table.get::<Option<i32>>("counter")?.unwrap_or(0),
                limit: table.get::<Option<i32>>("limit")?.unwrap_or(40),
            }),
            b"Freezer" => Ok(DynamicTerrainCell::Freezer {
                limit: table.get::<Option<i32>>("limit")?.unwrap_or(60), // note: ice spreads only half as far underwater
            }),
            b"Nitro" => Ok(DynamicTerrainCell::Nitro {
                counter: table.get::<Option<i32>>("counter")?.unwrap_or(0),
                limit: table.get::<Option<i32>>("limit")?.unwrap_or(5),
            }),
            b"Fire" => Ok(DynamicTerrainCell::Fire {
                counter: table.get::<Option<i32>>("counter")?.unwrap_or(60),
                cinder: false,
            }),
            t => Err(mlua::Error::FromLuaConversionError {
                from: "table",
                to: "DynamicTerrainCell".to_owned(),
                message: Some(format!(
                    "Unknown dynamic terrain type: {}",
                    str::from_utf8(t).unwrap()
                )),
            }),
        }
    }
}

pub(super) type DynamicTerrainMap = HashMap<(i32, i32), DynamicTerrainCell>;
