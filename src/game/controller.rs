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

use std::collections::HashMap;

use sdl3_sys::{events::SDL_KeyboardEvent, keycode::*};
use serde::{Deserialize, Serialize};

use crate::{configfile::GAME_CONFIG, events::push_menu_button_event};

/// How many controllers are reserved for keyboard use
const KEYBOARDS: usize = 4;

/**
 * The state of a single player's game controller.
 */
#[derive(Clone)]
pub struct GameController {
    pub thrust: bool,
    pub right: bool,
    pub left: bool,
    pub fire_primary: bool,
    pub fire_secondary: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerKeymap {
    pub thrust: u32,
    pub left: u32,
    pub right: u32,
    pub fire_primary: u32,
    pub fire_secondary: u32,
}

/**
 * Button events for controlling menus.
 *
 * Player ID 0 is used for the global non-configurable keys
 */
#[derive(Clone, Copy)]
pub enum MenuButton {
    None,
    Up(i32),
    Right(i32),
    Down(i32),
    Left(i32),
    Select(i32),
    Start,
    Back,
    Debug,
}

impl MenuButton {
    pub fn is_none(self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }

    pub fn to_event_code(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Up(p) => 0x010000 | p,
            Self::Right(p) => 0x020000 | p,
            Self::Down(p) => 0x030000 | p,
            Self::Left(p) => 0x040000 | p,
            Self::Select(p) => 0x050000 | p,
            Self::Start => 0x060000,
            Self::Back => 0x070000,
            Self::Debug => 0x080000,
        }
    }

    pub fn from_event_code(code: i32) -> MenuButton {
        match code & 0xff0000 {
            0x010000 => Self::Up(code & 0xffff),
            0x020000 => Self::Right(code & 0xffff),
            0x030000 => Self::Down(code & 0xffff),
            0x040000 => Self::Left(code & 0xffff),
            0x050000 => Self::Select(code & 0xffff),
            0x060000 => Self::Start,
            0x070000 => Self::Back,
            0x080000 => Self::Debug,
            _ => Self::None,
        }
    }
}
impl GameController {
    pub fn new() -> Self {
        Self {
            right: false,
            left: false,
            thrust: false,
            fire_primary: false,
            fire_secondary: false,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum MappedKey {
    Up,
    Right,
    Left,
    Fire1,
    Fire2,
}

pub struct GameControllerSet {
    /// List of game controllers
    /// The first four controllers are for keyboard inputs,
    /// gamepads are added to the end of the list.
    /// Disconnected gamepads will not be removed from the list
    /// so the indices will remain stable.
    pub states: Vec<GameController>,

    /// The last key up event keycode
    /// This is used for key grabbing
    pub last_grabbed_key: u32,
    key_grabbing: bool,

    keymap: HashMap<u32, (MappedKey, usize)>,
}

impl GameControllerSet {
    pub fn new() -> Self {
        Self {
            states: vec![GameController::new(); KEYBOARDS],
            keymap: HashMap::new(),
            last_grabbed_key: 0,
            key_grabbing: false,
        }
    }

    pub fn start_keygrab(&mut self) {
        self.last_grabbed_key = 0;
        self.key_grabbing = true;
    }

    pub fn reload_keymaps(&mut self) {
        let config = GAME_CONFIG.read().unwrap();

        self.keymap = HashMap::new();
        self.set_keymap(
            0,
            config.keymap1.as_ref().unwrap_or(&Self::DEFAULT_KEYMAP[0]),
        );
        self.set_keymap(
            1,
            config.keymap2.as_ref().unwrap_or(&Self::DEFAULT_KEYMAP[1]),
        );
        self.set_keymap(
            2,
            config.keymap3.as_ref().unwrap_or(&Self::DEFAULT_KEYMAP[2]),
        );
        self.set_keymap(
            3,
            config.keymap4.as_ref().unwrap_or(&Self::DEFAULT_KEYMAP[3]),
        );
    }

    pub fn get_keymap(&self, controller: usize) -> PlayerKeymap {
        assert!(controller < KEYBOARDS);

        let find_key = |k: MappedKey| -> u32 {
            *self
                .keymap
                .iter()
                .find(|(_, mapping)| mapping.0 == k && mapping.1 == controller)
                .map(|(k, _)| k)
                .unwrap_or(&0)
        };

        PlayerKeymap {
            thrust: find_key(MappedKey::Up),
            left: find_key(MappedKey::Left),
            right: find_key(MappedKey::Right),
            fire_primary: find_key(MappedKey::Fire1),
            fire_secondary: find_key(MappedKey::Fire2),
        }
    }

    fn set_keymap(&mut self, controller: usize, keymap: &PlayerKeymap) {
        self.keymap
            .insert(keymap.thrust, (MappedKey::Up, controller));
        self.keymap
            .insert(keymap.left, (MappedKey::Left, controller));
        self.keymap
            .insert(keymap.right, (MappedKey::Right, controller));
        self.keymap
            .insert(keymap.fire_primary, (MappedKey::Fire1, controller));
        self.keymap
            .insert(keymap.fire_secondary, (MappedKey::Fire2, controller));
    }

    pub fn handle_sdl_key_event(&mut self, key: &SDL_KeyboardEvent) {
        if self.key_grabbing && !key.down {
            self.last_grabbed_key = key.key;
            self.key_grabbing = false;
            return;
        }

        // Player key mappings
        let mut menubtn = MenuButton::None;

        if let Some((mapping, idx)) = self.keymap.get(&key.key) {
            let state = &mut self.states[*idx];
            match mapping {
                MappedKey::Up => {
                    state.thrust = key.down;
                    if !key.down {
                        menubtn = MenuButton::Up(*idx as i32 + 1);
                    }
                }
                MappedKey::Left => {
                    state.left = key.down;
                    if !key.down {
                        menubtn = MenuButton::Left(*idx as i32 + 1);
                    }
                }
                MappedKey::Right => {
                    state.right = key.down;
                    if !key.down {
                        menubtn = MenuButton::Right(*idx as i32 + 1);
                    }
                }
                MappedKey::Fire1 => {
                    state.fire_primary = key.down;
                    if !key.down {
                        menubtn = MenuButton::Select(*idx as i32 + 1);
                    }
                }
                MappedKey::Fire2 => state.fire_secondary = key.down,
            };
        }

        // Global "menu" keys (and key grabbing, which is also a menu thing)
        if !key.down && menubtn.is_none() {
            menubtn = match key.key {
                SDLK_UP => MenuButton::Up(0),
                SDLK_DOWN => MenuButton::Down(0),
                SDLK_LEFT => MenuButton::Left(0),
                SDLK_RIGHT => MenuButton::Right(0),
                SDLK_RETURN => MenuButton::Start,
                SDLK_ESCAPE => MenuButton::Back,
                SDLK_F12 => MenuButton::Debug,
                _ => MenuButton::None,
            }
        }

        if !menubtn.is_none() {
            push_menu_button_event(menubtn);
        }
    }

    pub const DEFAULT_KEYMAP: [PlayerKeymap; 4] = [
        PlayerKeymap {
            thrust: SDLK_UP,
            left: SDLK_LEFT,
            right: SDLK_RIGHT,
            fire_primary: SDLK_RSHIFT,
            fire_secondary: SDLK_RCTRL,
        },
        PlayerKeymap {
            thrust: SDLK_W,
            left: SDLK_A,
            right: SDLK_D,
            fire_primary: SDLK_LSHIFT,
            fire_secondary: SDLK_LCTRL,
        },
        PlayerKeymap {
            thrust: SDLK_KP_8,
            left: SDLK_KP_4,
            right: SDLK_KP_6,
            fire_primary: SDLK_KP_0,
            fire_secondary: SDLK_KP_ENTER,
        },
        PlayerKeymap {
            thrust: SDLK_I,
            left: SDLK_J,
            right: SDLK_L,
            fire_primary: SDLK_Y,
            fire_secondary: SDLK_H,
        },
    ];
}
