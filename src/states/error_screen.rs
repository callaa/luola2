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

use log::error;
use mlua::Either;
use std::{cell::RefCell, ffi::CString, rc::Rc};

use anyhow::Error;

use crate::{
    game::MenuButton,
    gfx::{Renderer, Text},
    math::Vec2,
    states::{StackableState, StackableStateResult},
};

pub struct ErrorScreenState {
    message: Either<Vec<CString>, Text>,
    renderer: Rc<RefCell<Renderer>>,
}

impl ErrorScreenState {
    pub fn new(err: Error, renderer: Rc<RefCell<Renderer>>) -> Self {
        let message = format!("{:?}", err);

        error!("Reached error state: {:?}", err);

        let message = match renderer
            .borrow()
            .try_fontset()
            .and_then(|fs| fs.menu.create_text(&renderer.borrow(), &message))
        {
            Ok(t) => Either::Right(t.with_wrapwidth(renderer.borrow().width())),
            Err(e) => {
                error!("Couldn't render error text! {e}");
                Either::Left(
                    message
                        .split('\n')
                        .map(|s| {
                            CString::new(s).expect("Couldn't turn error message into C string")
                        })
                        .collect(),
                )
            }
        };

        Self { message, renderer }
    }
}

impl StackableState for ErrorScreenState {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back => StackableStateResult::Pop,
            _ => StackableStateResult::Continue,
        }
    }

    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        error!(
            "Error screen received return value of type: {:?}",
            retval.type_id()
        );
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {}

    fn state_iterate(&mut self, _timestep: f32) -> StackableStateResult {
        let renderer = self.renderer.borrow();
        renderer.clear();

        match &self.message {
            Either::Left(lines) => lines
                .iter()
                .enumerate()
                .for_each(|(i, line)| renderer.draw_debug_text(line, 0.0, i as f32 * 8.0)),
            Either::Right(text) => text.render(Vec2::ZERO),
        }

        renderer.present();

        StackableStateResult::Continue
    }
}
