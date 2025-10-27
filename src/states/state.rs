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

use anyhow::Result;
use sdl3_main::AppResult;
use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{game::MenuButton, gfx::Renderer, states::ErrorScreenState};

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
    fn receive_return(&mut self, retval: Box<dyn Any>) -> Result<()>;
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

    pub fn handle_menu_button(&mut self, button: MenuButton) {
        if let Some(state) = self.states.last_mut() {
            match state.handle_menu_button(button) {
                StackableStateResult::Continue => {}
                StackableStateResult::Return(retval) => {
                    self.states.pop();
                    if let Err(err) = self
                        .states
                        .last_mut()
                        .expect("expected a state to return to")
                        .receive_return(retval)
                    {
                        self.states.clear();
                        self.states
                            .push(Box::new(ErrorScreenState::new(err, self.renderer.clone())));
                    }
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
    }

    pub fn state_iterate(&mut self, timestep: f32) -> AppResult {
        let result = if let Some(state) = self.states.last_mut() {
            state.state_iterate(timestep)
        } else {
            return AppResult::Success;
        };

        match result {
            StackableStateResult::Continue => {}
            StackableStateResult::Return(retval) => {
                self.states.pop();
                if let Err(err) = self
                    .states
                    .last_mut()
                    .expect("expected a state to return to")
                    .receive_return(retval)
                {
                    self.states.clear();
                    self.states
                        .push(Box::new(ErrorScreenState::new(err, self.renderer.clone())));
                }
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
        }

        if self.states.is_empty() {
            AppResult::Success
        } else {
            AppResult::Continue
        }
    }
}
