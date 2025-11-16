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

use serde::Deserialize;

use crate::game::hud::{HudOverlay, PlayerHud};
use crate::math::{Rect, Vec2};

#[derive(Deserialize)]
pub struct GameInitConfig {
    pub level: String,
    pub rounds: Option<i32>,
    pub gameover: Option<bool>,

    #[serde(rename = "player")]
    pub players: Vec<Player>,

    #[serde(default)]
    pub winners: Vec<PlayerId>,
}

#[derive(Deserialize, Clone)]
pub struct Player {
    /// Controller ID
    pub controller: i32,

    /// Ship name passed to init script
    pub ship: String,

    /// Special weapon name passed to init script
    pub weapon: String,

    /// Number of rounds won by this player
    #[serde(skip)]
    pub wins: i32,

    /// Viewport on screen
    #[serde(skip)]
    pub viewport: Rect,
}

pub type PlayerId = i32;

impl Player {
    pub fn new(controller: i32) -> Self {
        Self {
            controller,
            ship: String::new(),
            weapon: String::new(),
            wins: 0,
            viewport: Rect::new(0, 0, 1, 1),
        }
    }
}

/// Ingame state of a player
pub struct PlayerState {
    pub camera_pos: Vec2,
    pub hud: PlayerHud,
    pub overlays: Vec<HudOverlay>,

    /// Draw fadeout between 0..1
    pub fadeout: f32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            camera_pos: Vec2::ZERO,
            hud: PlayerHud::None,
            overlays: Vec::new(),
            fadeout: 0.0,
        }
    }
}
