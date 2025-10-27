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

use std::{ptr::null_mut, sync::LazyLock};

use sdl3_sys::events::{SDL_Event, SDL_PushEvent, SDL_RegisterEvents, SDL_UserEvent};

use crate::game::MenuButton;

#[derive(Debug)]
pub struct CustomEvents {
    pub grabkey: u32,
    pub config_changed: u32,
    pub menu_button: u32,
}

pub static CUSTOM_EVENTS: LazyLock<CustomEvents> = LazyLock::new(|| {
    let id = unsafe { SDL_RegisterEvents(3) };

    CustomEvents {
        grabkey: id,
        config_changed: id + 1,
        menu_button: id + 2,
    }
});

pub fn push_grabkey_event() {
    let mut ev = SDL_Event {
        r#type: CUSTOM_EVENTS.grabkey,
    };
    unsafe {
        SDL_PushEvent(&mut ev);
    }
}

pub fn push_config_changed_event() {
    let mut ev = SDL_Event {
        r#type: CUSTOM_EVENTS.config_changed,
    };
    unsafe {
        SDL_PushEvent(&mut ev);
    }
}

pub fn push_menu_button_event(button: MenuButton) {
    let mut ev = SDL_Event {
        user: SDL_UserEvent {
            r#type: CUSTOM_EVENTS.menu_button,
            reserved: 0,
            timestamp: 0,
            windowID: 0,
            code: button.to_event_code(),
            data1: null_mut(),
            data2: null_mut(),
        },
    };
    unsafe {
        SDL_PushEvent(&mut ev);
    }
}
