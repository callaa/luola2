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

use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{
    game::{MenuButton, PlayerId},
    gfx::{Color, RenderTextDest, RenderTextOptions, Renderer, Text, TextOutline},
    math::Vec2,
    menu::AnimatedStarfield,
    states::{StackableState, StackableStateResult},
};

pub struct RoundResultsState {
    renderer: Rc<RefCell<Renderer>>,
    starfield: Rc<RefCell<AnimatedStarfield>>,
    round_text: Text,
    winner_text: Text,
    timer: f32,
}

impl RoundResultsState {
    pub fn new(
        round_number: i32,
        winner: PlayerId,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let r = renderer.borrow();
        let font = &r.fontset().menu_big;

        let round_text = font
            .create_text(&r, &format!("Round {}", round_number))?
            .with_color(Color::new(0.9, 0.2, 0.2));

        let winner_text = if winner != 0 {
            font.create_text(&r, &format!("Player {winner} wins!"))?
                .with_color(Color::player_color(winner))
        } else {
            font.create_text(&r, "Draw!")?
                .with_color(Color::new(0.8, 0.8, 0.8))
        };

        drop(r);

        Ok(Self {
            renderer,
            starfield,
            round_text,
            winner_text,
            timer: 0.0,
        })
    }

    fn render(&self) {
        let r = self.renderer.borrow();

        r.clear();

        let fadein = if self.timer < 1.0 { self.timer } else { 1.0 };
        let fadeinout = if self.timer < 2.0 {
            fadein
        } else {
            3.0 - self.timer
        };

        self.starfield.borrow().render_with_alpha(&r, fadein);

        self.round_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(r.width() as f32 / 2.0, 10.0)),
            alpha: fadein,
            outline: TextOutline::Outline,
            ..Default::default()
        });
        self.winner_text.render(&RenderTextOptions {
            dest: RenderTextDest::Centered(Vec2(r.width() as f32 / 2.0, r.height() as f32 / 2.0)),
            alpha: fadeinout,
            ..Default::default()
        });

        r.present();
    }
}

impl StackableState for RoundResultsState {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back | MenuButton::Start => {
                return StackableStateResult::Pop;
            }
            _ => {}
        }
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        if self.timer < 3.0 {
            self.timer += timestep;
            self.starfield.borrow_mut().step(timestep);
            self.render();
            StackableStateResult::Continue
        } else {
            StackableStateResult::Pop
        }
    }
}
