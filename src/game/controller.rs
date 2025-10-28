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

use std::{collections::HashMap, ffi::CStr, ptr::null_mut};

use sdl3_sys::{
    events::SDL_KeyboardEvent,
    gamepad::{
        SDL_CloseGamepad, SDL_GAMEPAD_BUTTON_BACK, SDL_GAMEPAD_BUTTON_DPAD_DOWN,
        SDL_GAMEPAD_BUTTON_DPAD_LEFT, SDL_GAMEPAD_BUTTON_DPAD_RIGHT, SDL_GAMEPAD_BUTTON_DPAD_UP,
        SDL_GAMEPAD_BUTTON_EAST, SDL_GAMEPAD_BUTTON_SOUTH, SDL_GAMEPAD_BUTTON_START, SDL_Gamepad,
        SDL_GamepadButton, SDL_GamepadType, SDL_GetGamepadGUIDForID, SDL_GetGamepadStringForType,
        SDL_GetGamepadTypeForID, SDL_OpenGamepad, SDL_SetGamepadLED, SDL_SetGamepadPlayerIndex,
    },
    guid::SDL_GUID,
    joystick::{SDL_JOYSTICK_AXIS_MAX, SDL_JoystickID},
    keycode::*,
};
use serde::{Deserialize, Serialize};

use crate::{configfile::GAME_CONFIG, events::push_menu_button_event, gfx::Color};

/// How many controllers are reserved for keyboard use.
/// All other controllers are gamepads.
pub const KEYBOARDS: usize = 4;

/**
 * The state of a single player's game controller.
 */
#[derive(Clone)]
pub struct GameController {
    pub thrust: bool,
    pub turn: f32,
    pub fire_primary: bool,
    pub fire_secondary: bool,

    guid: SDL_GUID,
    joystick_id: SDL_JoystickID,
    gamepad: *mut SDL_Gamepad,
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
            turn: 0.0,
            thrust: false,
            fire_primary: false,
            fire_secondary: false,
            guid: SDL_GUID { data: [0; 16] },
            joystick_id: 0,
            gamepad: null_mut(),
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

    pub fn add_gamepad(&mut self, id: SDL_JoystickID) {
        let guid = unsafe { SDL_GetGamepadGUIDForID(id) };
        let gamepad = unsafe { SDL_OpenGamepad(id) };
        if gamepad.is_null() {
            log::error!("Couldn't open gamepad {id}!");
            return;
        }

        let ctrl = self
            .states
            .iter_mut()
            .enumerate()
            .skip(KEYBOARDS)
            .find(|c| c.1.gamepad.is_null() && c.1.guid.data == guid.data);

        let typestr =
            unsafe { CStr::from_ptr(SDL_GetGamepadStringForType(SDL_GetGamepadTypeForID(id))) };

        if let Some((idx, ctrl)) = ctrl {
            // Re-use an unplugged controller slot with the same GUID (if any)
            log::info!(
                "Gamepad {id} {:?} reconnected as controller #{}.",
                typestr,
                idx + 1
            );
            ctrl.joystick_id = id;
            ctrl.gamepad = gamepad;
        } else {
            // Add a new controller otherwise.
            self.states.push(GameController {
                guid,
                joystick_id: id,
                gamepad,
                ..GameController::new()
            });
            log::info!(
                "Gamepad {id} {:?} added as controller #{}.",
                typestr,
                self.states.len()
            );
        }
    }

    pub fn remove_gamepad(&mut self, id: SDL_JoystickID) {
        let ctrl = self
            .states
            .iter_mut()
            .enumerate()
            .skip(KEYBOARDS)
            .find(|c| c.1.joystick_id == id);
        if let Some((idx, ctrl)) = ctrl {
            log::info!("Gamepad {id} (controller #{}) removed.", idx + 1);
            unsafe {
                SDL_CloseGamepad(ctrl.gamepad);
            }
            ctrl.gamepad = null_mut();
        } else {
            log::info!("Unknown gamepad {id} removed.");
        }
    }

    pub fn get_gamepad_type(&self, controller: i32) -> SDL_GamepadType {
        let ctrl = &self.states[controller as usize - 1];
        if ctrl.gamepad.is_null() {
            return SDL_GamepadType::UNKNOWN;
        }

        unsafe { SDL_GetGamepadTypeForID(ctrl.joystick_id) }
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
                    state.turn = if key.down { 1.0 } else { 0.0 };
                    if !key.down {
                        menubtn = MenuButton::Left(*idx as i32 + 1);
                    }
                }
                MappedKey::Right => {
                    state.turn = if key.down { -1.0 } else { 0.0 };
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

    pub fn handle_gamepad_axis(&mut self, id: SDL_JoystickID, axis: u8, value: i16) {
        let state = match self
            .states
            .iter_mut()
            .skip(KEYBOARDS)
            .find(|s| s.joystick_id == id)
        {
            Some(s) => s,
            None => {
                log::error!("Gamepad axis event for unknown gamepad {id}");
                return;
            }
        };

        // Deadzone
        // TODO this should be configurable
        let value = value as f32 / SDL_JOYSTICK_AXIS_MAX as f32;
        let value = if value.abs() < 8000 as f32 / SDL_JOYSTICK_AXIS_MAX as f32 {
            0.0
        } else {
            value
        };

        if axis == 0 {
            state.turn = -value;
        }

        /* TODO make this configurable
        if axis == 1 {
            state.thrust = value < 0.0;
        }
        */
        if axis == 4 {
            state.thrust = value > 0.0;
        }
    }

    pub fn handle_gamepad_button(
        &mut self,
        id: SDL_JoystickID,
        button: SDL_GamepadButton,
        down: bool,
    ) {
        let (ctrl_id, state) = match self
            .states
            .iter_mut()
            .enumerate()
            .skip(KEYBOARDS)
            .find(|s| s.1.joystick_id == id)
        {
            Some(s) => (s.0 as i32 + 1, s.1),
            None => {
                log::error!("Gamepad button event for unknown gamepad {id}");
                return;
            }
        };

        let mut menubtn = MenuButton::None;

        match button {
            SDL_GAMEPAD_BUTTON_DPAD_UP => {
                if down {
                    menubtn = MenuButton::Up(ctrl_id);
                }
            }
            SDL_GAMEPAD_BUTTON_DPAD_RIGHT => {
                if down {
                    menubtn = MenuButton::Right(ctrl_id);
                }
            }
            SDL_GAMEPAD_BUTTON_DPAD_DOWN => {
                if down {
                    menubtn = MenuButton::Down(ctrl_id);
                }
            }
            SDL_GAMEPAD_BUTTON_DPAD_LEFT => {
                if down {
                    menubtn = MenuButton::Left(ctrl_id);
                }
            }
            SDL_GAMEPAD_BUTTON_EAST => {
                if down {
                    menubtn = MenuButton::Select(ctrl_id);
                }
                state.fire_primary = down;
            }
            SDL_GAMEPAD_BUTTON_SOUTH => {
                state.fire_secondary = down;
            }
            SDL_GAMEPAD_BUTTON_START => {
                if down {
                    menubtn = MenuButton::Start;
                }
            }
            SDL_GAMEPAD_BUTTON_BACK => {
                if down {
                    menubtn = MenuButton::Back;
                }
            }
            _ => {}
        }

        if !menubtn.is_none() {
            push_menu_button_event(menubtn);
        }
    }

    /// Set the LEDs on the controller to indicate the player number and color
    pub fn set_player_leds(&self, controller_id: i32, player_id: i32) {
        if controller_id <= KEYBOARDS as i32 {
            // TODO RGB keyboard support would be cool
            return;
        }
        let ctrl = &self.states[controller_id as usize - 1];
        if !ctrl.gamepad.is_null() {
            let color = Color::player_color(player_id);
            unsafe {
                SDL_SetGamepadLED(ctrl.gamepad, color.r_u8(), color.g_u8(), color.b_u8());
                SDL_SetGamepadPlayerIndex(ctrl.gamepad, player_id - 1);
            }
        }
    }

    /// Turn off all player LEDs
    pub fn clear_player_leds(&self) {
        for ctrl in self.states.iter().skip(KEYBOARDS) {
            if !ctrl.gamepad.is_null() {
                unsafe {
                    SDL_SetGamepadLED(ctrl.gamepad, 0, 0, 0);
                    SDL_SetGamepadPlayerIndex(ctrl.gamepad, -1);
                }
            }
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
