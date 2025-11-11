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

#![allow(dead_code)] // TODO remove later

use argh::FromArgs;
use log::error;
use sdl3_main::{AppResult, AppResultWithState, app_impl};
use sdl3_sys::events::{
    SDL_EVENT_GAMEPAD_ADDED, SDL_EVENT_GAMEPAD_AXIS_MOTION, SDL_EVENT_GAMEPAD_BUTTON_DOWN,
    SDL_EVENT_GAMEPAD_BUTTON_UP, SDL_EVENT_GAMEPAD_REMOVED, SDL_EVENT_KEY_DOWN, SDL_EVENT_KEY_UP,
    SDL_EVENT_QUIT, SDL_EVENT_USER, SDL_EVENT_WINDOW_RESIZED, SDL_Event, SDL_EventType,
};
use sdl3_sys::gamepad::{SDL_GamepadAxis, SDL_GamepadButton};
use sdl3_sys::init::{SDL_INIT_GAMEPAD, SDL_INIT_VIDEO, SDL_Init, SDL_SetAppMetadata};
use sdl3_sys::keycode::{SDL_KMOD_ALT, SDLK_RETURN};
use sdl3_sys::timer::{SDL_DelayNS, SDL_GetTicksNS};

use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Mutex;

use crate::configfile::{GAME_CONFIG, load_user_config};
use crate::events::CUSTOM_EVENTS;
use crate::game::{GameControllerSet, MenuButton};
use crate::gfx::{Renderer, SdlError};
use crate::states::{GameInitState, StateStack};

mod configfile;
pub mod events;
mod fs;
mod game;
mod gfx;
mod math;
mod menu;
mod states;

struct AppState {
    renderer: Rc<RefCell<Renderer>>,
    controllers: Rc<RefCell<GameControllerSet>>,
    statestack: StateStack,
}

#[derive(FromArgs)]
#[argh(description = "Cave Flying Action Game")]
struct Arguments {
    #[argh(option, description = "launch directly from configuration")]
    launch: Option<String>,

    #[argh(switch, short = 'f', description = "start in fullscreen mode")]
    fullscreen: bool,

    #[argh(switch, short = 'w', description = "start in windowed mode")]
    window: bool,
}

unsafe impl Send for AppState {}

#[app_impl]
impl AppState {
    fn app_init() -> AppResultWithState<Box<Mutex<Self>>> {
        let args: Arguments = argh::from_env();

        env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .init();

        unsafe {
            if !SDL_SetAppMetadata(
                c"Luola II".as_ptr(),
                CString::new(env!("CARGO_PKG_VERSION")).unwrap().as_ptr(),
                c"io.github.callaa.luola2".as_ptr(),
            ) {
                return AppResultWithState::Failure(None);
            }

            if !SDL_Init(SDL_INIT_VIDEO | SDL_INIT_GAMEPAD) {
                SdlError::log("Couldn't init SDL");
                return AppResultWithState::Failure(None);
            }
        }

        load_user_config();
        let config = GAME_CONFIG.read().unwrap();

        let renderer =
            match Renderer::create(!args.window && (args.fullscreen || config.video.fullscreen)) {
                Ok(r) => Rc::new(RefCell::new(r)),
                Err(err) => {
                    error!("Couldn't create renderer: {}", err);
                    return AppResultWithState::Failure(None);
                }
            };

        let mut controllers = GameControllerSet::new();
        controllers.reload_keymaps();

        let controllers = Rc::new(RefCell::new(controllers));

        let mut statestack = StateStack::new(renderer.clone());
        statestack.push(Box::new(GameInitState::new(
            args.launch,
            controllers.clone(),
            renderer.clone(),
        )));

        AppResultWithState::Continue(Box::new(Mutex::new(AppState {
            renderer,
            controllers,
            statestack,
        })))
    }

    fn app_iterate(&mut self) -> AppResult {
        let ticks = unsafe { SDL_GetTicksNS() };
        let result = self.statestack.state_iterate(1.0 / 60.0);
        let ticks2 = unsafe { SDL_GetTicksNS() };

        // Limit framerate
        let dticks = ticks2 - ticks;
        if dticks < NANOSECONDS_PER_FRAME {
            unsafe {
                SDL_DelayNS(NANOSECONDS_PER_FRAME - dticks);
            }
        }

        result
    }

    fn app_event(&mut self, event: &SDL_Event) -> AppResult {
        let event_type = SDL_EventType(unsafe { event.r#type });
        match event_type {
            SDL_EVENT_QUIT => return AppResult::Success,
            SDL_EVENT_WINDOW_RESIZED => {
                if let Err(e) = self.renderer.borrow_mut().reset_viewport() {
                    error!("Failed to handle window resize: {}", e);
                } else {
                    self.statestack.resize_screen();
                }
            }
            SDL_EVENT_KEY_DOWN | SDL_EVENT_KEY_UP => {
                let key = unsafe { &event.key };
                if key.key == SDLK_RETURN && (key.r#mod & SDL_KMOD_ALT) > 0 && !key.down {
                    self.renderer.borrow_mut().toggle_fullscreen();
                } else {
                    self.controllers.borrow_mut().handle_sdl_key_event(key);
                }
            }
            SDL_EVENT_GAMEPAD_AXIS_MOTION => {
                let event = unsafe { &event.gaxis };
                self.controllers.borrow_mut().handle_gamepad_axis(
                    event.which,
                    SDL_GamepadAxis(event.axis as i32),
                    event.value,
                );
            }
            SDL_EVENT_GAMEPAD_BUTTON_DOWN | SDL_EVENT_GAMEPAD_BUTTON_UP => {
                let event = unsafe { &event.gbutton };
                self.controllers.borrow_mut().handle_gamepad_button(
                    event.which,
                    SDL_GamepadButton(event.button as i32),
                    event.down,
                );
            }
            SDL_EVENT_GAMEPAD_ADDED => {
                let event = unsafe { &event.gdevice };
                self.controllers.borrow_mut().add_gamepad(event.which);
            }
            SDL_EVENT_GAMEPAD_REMOVED => {
                let event = unsafe { &event.gdevice };
                self.controllers.borrow_mut().remove_gamepad(event.which);
            }
            t if t >= SDL_EVENT_USER => {
                let custom = &CUSTOM_EVENTS;
                if t.0 == custom.grabkey {
                    self.controllers.borrow_mut().start_keygrab();
                } else if t.0 == custom.config_changed {
                    self.controllers.borrow_mut().reload_keymaps();
                } else if t.0 == custom.menu_button {
                    let userev = unsafe { &event.user };
                    self.statestack
                        .handle_menu_button(MenuButton::from_event_code(userev.code));
                }
            }
            _ => {}
        }

        AppResult::Continue
    }
}

// weirdness: if I use const here instead of static, rustc (1.90.0) and rust-analyzer will consume
// all memory and crash
static NANOSECONDS_PER_FRAME: u64 = 1_000_000_000 / 60;
