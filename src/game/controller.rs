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
    collections::HashMap,
    ffi::{CStr, c_void},
    ptr::{self, null_mut},
};

use sdl3_sys::{
    events::SDL_KeyboardEvent,
    gamepad::{
        SDL_CloseGamepad, SDL_GAMEPAD_AXIS_LEFT_TRIGGER, SDL_GAMEPAD_AXIS_LEFTX,
        SDL_GAMEPAD_AXIS_LEFTY, SDL_GAMEPAD_AXIS_RIGHT_TRIGGER, SDL_GAMEPAD_AXIS_RIGHTX,
        SDL_GAMEPAD_AXIS_RIGHTY, SDL_GAMEPAD_BUTTON_BACK, SDL_GAMEPAD_BUTTON_DPAD_DOWN,
        SDL_GAMEPAD_BUTTON_DPAD_LEFT, SDL_GAMEPAD_BUTTON_DPAD_RIGHT, SDL_GAMEPAD_BUTTON_DPAD_UP,
        SDL_GAMEPAD_BUTTON_EAST, SDL_GAMEPAD_BUTTON_LEFT_SHOULDER, SDL_GAMEPAD_BUTTON_NORTH,
        SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER, SDL_GAMEPAD_BUTTON_SOUTH, SDL_GAMEPAD_BUTTON_START,
        SDL_GAMEPAD_BUTTON_WEST, SDL_Gamepad, SDL_GamepadAxis, SDL_GamepadButton, SDL_GamepadType,
        SDL_GetGamepadAxis, SDL_GetGamepadButton, SDL_GetGamepadGUIDForID,
        SDL_GetGamepadStringForType, SDL_GetGamepadTypeForID, SDL_OpenGamepad, SDL_RumbleGamepad,
        SDL_SetGamepadLED, SDL_SetGamepadPlayerIndex,
    },
    guid::SDL_GUID,
    joystick::{SDL_JOYSTICK_AXIS_MAX, SDL_JoystickID},
    keycode::*,
};
use serde::{Deserialize, Serialize};

use crate::{configfile::GAME_CONFIG, events::push_menu_button_event, game::PlayerId, gfx::Color};

/// How many controllers are reserved for keyboard use.
/// All other controllers are gamepads.
pub const KEYBOARDS: usize = 4;

/**
 * The state of a single player's game controller.
 */
#[derive(Clone)]
pub struct GameController {
    pub thrust: f32,
    pub walk: f32, // left thumbstick X-axis: same as turn on keyboards
    pub turn: f32,
    pub aim: f32, // right thumbstick Y-axis: shortcut for aim-mode + thrust up/down on gamepads only
    pub jump: bool, // same as thrust>0 on keyboards
    pub fire1: bool,
    pub fire2: bool,
    pub fire3: bool,
    pub eject: bool, // same as thrust<0 & fire_primary on keyboards

    guid: SDL_GUID,
    joystick_id: SDL_JoystickID,
    gamepad: *mut SDL_Gamepad,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerKeymap {
    pub thrust: u32,
    pub down: u32,
    pub left: u32,
    pub right: u32,
    pub fire1: u32,
    pub fire2: u32,
    pub fire3: u32,
}

/**
 * Button events for controlling menus.
 *
 * Player ID 0 is used for the global non-configurable keys
 */
#[derive(Clone, Copy)]
pub enum MenuButton {
    None,
    Up(PlayerId),
    Right(PlayerId),
    Down(PlayerId),
    Left(PlayerId),
    Select(PlayerId),
    Start,
    Back,
    Debug,
    GrabbedKey(u32),
}

impl MenuButton {
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Return a (code, data1) pair to be used in a custom SDL event
    pub fn to_event_code(self) -> (i32, *mut c_void) {
        match self {
            Self::None => (0, null_mut()),
            Self::Up(p) => (1, ptr::without_provenance_mut(p as usize)),
            Self::Right(p) => (2, ptr::without_provenance_mut(p as usize)),
            Self::Down(p) => (3, ptr::without_provenance_mut(p as usize)),
            Self::Left(p) => (4, ptr::without_provenance_mut(p as usize)),
            Self::Select(p) => (5, ptr::without_provenance_mut(p as usize)),
            Self::Start => (6, null_mut()),
            Self::Back => (7, null_mut()),
            Self::Debug => (8, null_mut()),
            Self::GrabbedKey(k) => (9, ptr::without_provenance_mut(k as usize)),
        }
    }

    pub fn from_event_code(code: i32, data1: *mut c_void) -> MenuButton {
        match code {
            1 => Self::Up(data1 as PlayerId),
            2 => Self::Right(data1 as PlayerId),
            3 => Self::Down(data1 as PlayerId),
            4 => Self::Left(data1 as PlayerId),
            5 => Self::Select(data1 as PlayerId),
            6 => Self::Start,
            7 => Self::Back,
            8 => Self::Debug,
            9 => Self::GrabbedKey(data1 as u32),
            _ => Self::None,
        }
    }
}
impl GameController {
    pub fn new() -> Self {
        Self {
            thrust: 0.0,
            walk: 0.0,
            turn: 0.0,
            aim: 0.0,
            fire1: false,
            fire2: false,
            fire3: false,
            eject: false,
            jump: false,
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
    Down,
    Left,
    Fire1,
    Fire2,
    Fire3,
}

pub struct GameControllerSet {
    /// List of game controllers
    /// The first four controllers are for keyboard inputs,
    /// gamepads are added to the end of the list.
    /// Disconnected gamepads will not be removed from the list
    /// so the indices will remain stable.
    pub states: Vec<GameController>,

    /// In key grab mode, the next button press will emit a GrabbedKey menu button event
    key_grabbing: bool,

    keymap: HashMap<u32, (MappedKey, usize)>,
}

impl GameControllerSet {
    pub fn new() -> Self {
        Self {
            states: vec![GameController::new(); KEYBOARDS],
            keymap: HashMap::new(),
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
        self.key_grabbing = true;
        log::debug!("Started keygrab mode");
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
            down: find_key(MappedKey::Down),
            left: find_key(MappedKey::Left),
            right: find_key(MappedKey::Right),
            fire1: find_key(MappedKey::Fire1),
            fire2: find_key(MappedKey::Fire2),
            fire3: find_key(MappedKey::Fire3),
        }
    }

    fn set_keymap(&mut self, controller: usize, keymap: &PlayerKeymap) {
        self.keymap
            .insert(keymap.thrust, (MappedKey::Up, controller));
        self.keymap
            .insert(keymap.down, (MappedKey::Down, controller));
        self.keymap
            .insert(keymap.left, (MappedKey::Left, controller));
        self.keymap
            .insert(keymap.right, (MappedKey::Right, controller));
        self.keymap
            .insert(keymap.fire1, (MappedKey::Fire1, controller));
        self.keymap
            .insert(keymap.fire2, (MappedKey::Fire2, controller));
        self.keymap
            .insert(keymap.fire3, (MappedKey::Fire3, controller));
    }

    pub fn handle_sdl_key_event(&mut self, key: &SDL_KeyboardEvent) {
        if self.key_grabbing && !key.down {
            self.key_grabbing = false;
            log::debug!("Grabbed key 0x{:x}", key.key);
            push_menu_button_event(MenuButton::GrabbedKey(key.key));
            return;
        }

        // Player key mappings
        let mut menubtn = MenuButton::None;

        if let Some((mapping, idx)) = self.keymap.get(&key.key) {
            let state = &mut self.states[*idx];
            match mapping {
                MappedKey::Up => {
                    state.thrust = if key.down { 1.0 } else { 0.0 };
                    state.jump = key.down;
                    if !key.down {
                        menubtn = MenuButton::Up(*idx as i32 + 1);
                    }
                }
                MappedKey::Down => {
                    state.thrust = if key.down { -1.0 } else { 0.0 };
                    state.eject = state.fire3 & (state.thrust < 0.0);
                    if !key.down {
                        menubtn = MenuButton::Down(*idx as i32 + 1);
                    }
                }
                MappedKey::Left => {
                    state.turn = if key.down { 1.0 } else { 0.0 };
                    state.walk = state.turn;
                    if !key.down {
                        menubtn = MenuButton::Left(*idx as i32 + 1);
                    }
                }
                MappedKey::Right => {
                    state.turn = if key.down { -1.0 } else { 0.0 };
                    state.walk = state.turn;
                    if !key.down {
                        menubtn = MenuButton::Right(*idx as i32 + 1);
                    }
                }
                MappedKey::Fire1 => {
                    state.fire1 = key.down;
                    if !key.down {
                        menubtn = MenuButton::Select(*idx as i32 + 1);
                    }
                }
                MappedKey::Fire2 => state.fire2 = key.down,
                MappedKey::Fire3 => {
                    state.fire3 = key.down;
                    state.eject = state.fire3 & (state.thrust < 0.0);
                }
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

    pub fn handle_gamepad_axis(&mut self, id: SDL_JoystickID, axis: SDL_GamepadAxis, value: i16) {
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

        let value = Self::axis_value(value);

        if axis == SDL_GAMEPAD_AXIS_RIGHTX {
            // Buff turning speed for gamepad users, since the thumb stick requires a bigger
            // motion compared to a key press
            state.turn = value * -1.15;
        } else if axis == SDL_GAMEPAD_AXIS_RIGHTY {
            // No equivalent for this on keyboard.
            // Only aim-mode + thrust combo available there.
            state.aim = -value;
        } else if axis == SDL_GAMEPAD_AXIS_LEFTY {
            state.thrust = -value;
        } else if axis == SDL_GAMEPAD_AXIS_LEFTX {
            state.walk = -value;
        } else if axis == SDL_GAMEPAD_AXIS_RIGHT_TRIGGER {
            // primary fire: also east button
            let firebtn = unsafe { SDL_GetGamepadButton(state.gamepad, SDL_GAMEPAD_BUTTON_EAST) };
            state.fire1 = firebtn || value > 0.0;
        } else if axis == SDL_GAMEPAD_AXIS_LEFT_TRIGGER {
            // secondary fire: also south button
            let firebtn = unsafe { SDL_GetGamepadButton(state.gamepad, SDL_GAMEPAD_BUTTON_SOUTH) };
            state.fire2 = firebtn || value > 0.0;
        }
    }

    fn axis_value(val: i16) -> f32 {
        // TODO deadzone should be configurable
        let value = val as f32 / SDL_JOYSTICK_AXIS_MAX as f32;
        if value.abs() < 8000.0 / SDL_JOYSTICK_AXIS_MAX as f32 {
            0.0
        } else {
            value
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
                let axis = Self::axis_value(unsafe {
                    SDL_GetGamepadAxis(state.gamepad, SDL_GAMEPAD_AXIS_RIGHT_TRIGGER)
                });
                state.fire1 = down || axis > 0.0;
            }
            SDL_GAMEPAD_BUTTON_SOUTH => {
                state.fire2 = down;
                let axis = Self::axis_value(unsafe {
                    SDL_GetGamepadAxis(state.gamepad, SDL_GAMEPAD_AXIS_LEFT_TRIGGER)
                });
                state.fire2 = down || axis > 0.0;
            }
            SDL_GAMEPAD_BUTTON_WEST => {
                state.fire3 = down;
            }
            SDL_GAMEPAD_BUTTON_NORTH => {
                state.jump = down;
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
            SDL_GAMEPAD_BUTTON_LEFT_SHOULDER | SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER => {
                state.eject = unsafe {
                    SDL_GetGamepadButton(state.gamepad, SDL_GAMEPAD_BUTTON_LEFT_SHOULDER)
                        & SDL_GetGamepadButton(state.gamepad, SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER)
                };
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

    pub fn rumble(&self, controller_id: i32, low_freq: f32, high_freq: f32, duration: f32) {
        debug_assert!(duration >= 0.0);

        if controller_id <= KEYBOARDS as i32 || controller_id > self.states.len() as i32 {
            return;
        }

        let ctrl = &self.states[controller_id as usize - 1];
        if !ctrl.gamepad.is_null() {
            const MAX_FREQ: f32 = 0xffff as _;

            unsafe {
                SDL_RumbleGamepad(
                    ctrl.gamepad,
                    (low_freq * MAX_FREQ).clamp(0.0, MAX_FREQ) as u16,
                    (high_freq * MAX_FREQ).clamp(0.0, MAX_FREQ) as u16,
                    (duration * 1000.0) as u32,
                );
            }
        }
    }

    pub const DEFAULT_KEYMAP: [PlayerKeymap; 4] = [
        PlayerKeymap {
            thrust: SDLK_UP,
            down: SDLK_DOWN,
            left: SDLK_LEFT,
            right: SDLK_RIGHT,
            fire1: SDLK_RSHIFT,
            fire2: SDLK_RCTRL,
            fire3: SDLK_MINUS,
        },
        PlayerKeymap {
            thrust: SDLK_W,
            down: SDLK_S,
            left: SDLK_A,
            right: SDLK_D,
            fire1: SDLK_LSHIFT,
            fire2: SDLK_LCTRL,
            fire3: SDLK_Q,
        },
        PlayerKeymap {
            thrust: SDLK_KP_8,
            down: SDLK_KP_5,
            left: SDLK_KP_4,
            right: SDLK_KP_6,
            fire1: SDLK_KP_0,
            fire2: SDLK_KP_ENTER,
            fire3: SDLK_KP_1, // TODO
        },
        PlayerKeymap {
            thrust: SDLK_I,
            down: SDLK_K,
            left: SDLK_J,
            right: SDLK_L,
            fire1: SDLK_Y,
            fire2: SDLK_H,
            fire3: SDLK_U,
        },
    ];
}
