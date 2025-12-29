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

use anyhow::anyhow;
use log::error;
use sdl3_main::AppResult;
use std::{any::Any, cell::RefCell, rc::Rc, time::SystemTime};

use crate::{
    fs::get_screenshot_path,
    game::MenuButton,
    gfx::{Color, Renderer},
    math::RectF,
    states::ErrorScreenState,
};

pub enum StackableStateResult {
    Continue,
    Replace(Box<dyn StackableState>),
    Push(Box<dyn StackableState>),
    Return(Box<dyn Any>),
    Pop,
    Error(anyhow::Error),
}

// Note: when FromResidual is no longer experimental,
// we could implement it for StackableStateResult

pub struct StateStack {
    states: Vec<Box<dyn StackableState>>,
    renderer: Rc<RefCell<Renderer>>,
}

pub trait StackableState {
    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        error!(
            "State with no receive_return implemented received unexpected return type {:?}",
            (*retval).type_id()
        );

        StackableStateResult::Error(anyhow!("Unexpected return with value!"))
    }

    fn resize_screen(&mut self);
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult;
    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult;
}

impl StateStack {
    pub fn new(renderer: Rc<RefCell<Renderer>>) -> Self {
        Self {
            states: Vec::new(),
            renderer,
        }
    }

    pub fn push(&mut self, state: Box<dyn StackableState>) {
        self.states.push(state);
    }

    pub fn resize_screen(&mut self) {
        for state in self.states.iter_mut() {
            state.resize_screen();
        }
    }

    fn handle_state_result(&mut self, result: StackableStateResult) {
        match result {
            StackableStateResult::Continue => {}
            StackableStateResult::Return(retval) => {
                self.states.pop();
                let result = self
                    .states
                    .last_mut()
                    .expect("expected a state to return to")
                    .receive_return(retval);

                self.handle_state_result(result);
            }
            StackableStateResult::Pop => {
                self.states.pop();
            }
            StackableStateResult::Replace(s) => {
                self.states.pop();
                self.states.push(s)
            }
            StackableStateResult::Push(s) => self.states.push(s),
            StackableStateResult::Error(err) => {
                self.states.clear();
                self.states
                    .push(Box::new(ErrorScreenState::new(err, self.renderer.clone())));
            }
        };
    }

    pub fn handle_menu_button(&mut self, button: MenuButton) {
        if matches!(button, MenuButton::Screenshot) {
            if let Err(e) = self.take_screenshot() {
                log::warn!("Couldn't save screenshot: {e}");
            }
        } else {
            let result = match self.states.last_mut() {
                Some(s) => s.handle_menu_button(button),
                None => return,
            };
            self.handle_state_result(result);
        }
    }

    pub fn state_iterate(&mut self, timestep: f32) -> AppResult {
        let result = if let Some(state) = self.states.last_mut() {
            state.state_iterate(timestep)
        } else {
            return AppResult::Success;
        };

        self.handle_state_result(result);

        if self.states.is_empty() {
            AppResult::Success
        } else {
            AppResult::Continue
        }
    }

    fn take_screenshot(&self) -> anyhow::Result<()> {
        let image = self.renderer.borrow().screenshot()?;
        let mut path = get_screenshot_path()?;
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid time expected");
        path.push(format!("luola2-{}.bmp", ts.as_secs()));

        log::info!("Saving screenshot to: {:?}", path);
        image.save_bmp(path)?;

        // Flash the screen to indicate a screenshot was taken
        let r = self.renderer.borrow_mut();
        r.draw_filled_rectangle(
            RectF::new(0.0, 0.0, r.width() as f32, r.height() as f32),
            &Color::WHITE,
        );
        r.present();
        Ok(())
    }
}
