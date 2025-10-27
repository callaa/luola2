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

use crate::game::level::LevelInfo;

/// Game assets (levels, weapons, etc.) loaded in the beginning
pub struct GameAssets {
    pub levels: Vec<LevelInfo>,
    pub weapons: Vec<(String, String)>,
}

impl GameAssets {
    pub fn new() -> Self {
        Self {
            levels: Vec::new(),
            weapons: Vec::new(),
        }
    }
}
