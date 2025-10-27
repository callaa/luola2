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
    game::{MenuButton, Player, PlayerId},
    gfx::{Color, Renderer, Text},
    math::Vec2,
    menu::AnimatedStarfield,
    states::{StackableState, StackableStateResult},
};

pub struct GameResultsState {
    players: Vec<Player>,
    round_winners: Vec<PlayerId>,
    renderer: Rc<RefCell<Renderer>>,

    starfield: AnimatedStarfield,
    gameover_text: Text,
    player_numbers: Vec<Text>,
    player_results: Vec<(i32, i32, Text)>,
    winner_text: Text,

    rounds_shown_anim: usize,
    results_shown_anim: usize,
    results_box_size: (f32, f32),
    timer: f32,
    exit_timer: Option<f32>,
}

impl GameResultsState {
    pub fn new(
        players: Vec<Player>,
        round_winners: Vec<PlayerId>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let r = renderer.borrow();

        let starfield = AnimatedStarfield::new(200, r.width() as f32, r.height() as f32);

        let mut player_numbers = Vec::with_capacity(players.len() + 1);
        player_numbers.push(
            r.fontset()
                .menu
                .create_text(&r, "X")?
                .with_color(Color::new(0.6, 0.6, 0.6)),
        );

        for idx in 1..=players.len() as i32 {
            player_numbers.push(
                r.fontset()
                    .menu
                    .create_text(&r, &format!("{}", idx))?
                    .with_color(Color::player_color(idx)),
            );
        }

        let mut player_results = players
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                r.fontset()
                    .menu
                    .create_text(&r, &format!("Player {} - {}", idx + 1, p.wins))
                    .map(|t| {
                        (
                            p.wins,
                            idx as i32 + 1,
                            t.with_color(Color::player_color(idx as i32 + 1)),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        player_results.sort_by(|a, b| b.0.cmp(&a.0));

        let results_box_size = player_results
            .iter()
            .map(|(_, _, txt)| (txt.width(), txt.height()))
            .reduce(|acc, (w, h)| (acc.0.max(w), acc.1 + h))
            .expect("There should be at least one player");

        let gameover_text = r
            .fontset()
            .menu_big
            .create_text(&r, "Game Over!")?
            .with_color(Color::new(0.9, 0.1, 0.1));

        let is_draw = player_results[0].0 == 0;

        let winner_text = if is_draw {
            r.fontset()
                .menu_big
                .create_text(&r, "Nobody wins!")?
                .with_color(Color::new(0.5, 0.5, 0.5))
        } else {
            r.fontset()
                .menu_big
                .create_text(&r, &format!("Player {} wins!", player_results[0].1))?
                .with_color(Color::player_color(player_results[0].1))
        };

        let results_box_size = (
            results_box_size.0,
            results_box_size.1 + winner_text.height(),
        );

        drop(r);

        Ok(Self {
            players,
            round_winners,
            renderer,
            starfield,
            gameover_text,
            player_numbers,
            player_results,
            winner_text,
            results_box_size,
            rounds_shown_anim: 0,
            results_shown_anim: 0,
            timer: 0.0,
            exit_timer: None,
        })
    }

    fn render(&self) {
        let r = self.renderer.borrow();

        r.clear();

        self.starfield.render(&r);

        const SPACING: f32 = 5.0;

        // Heading
        if self.results_shown_anim <= self.player_results.len() {
            self.gameover_text.render_hcenter(r.width() as f32, SPACING);
        }

        // Round result table
        let rounds_width =
            (self.player_numbers[0].width() + SPACING) * self.round_winners.len() as f32;

        let mut rounds_x = (r.width() as f32 - rounds_width) / 2.0;
        let rounds_y = self.gameover_text.height() + SPACING * 3.0;

        for r in self.round_winners.iter().take(self.rounds_shown_anim) {
            let tex = &self.player_numbers[*r as usize];
            tex.render(Vec2(rounds_x, rounds_y));
            rounds_x += tex.width() + SPACING;
        }

        // Player ranking
        let x = (r.width() as f32 - self.results_box_size.0) / 2.0;
        let mut y = (r.height() as f32 - self.results_box_size.1) / 2.0 + self.results_box_size.1;

        for (_, _, res) in self
            .player_results
            .iter()
            .rev()
            .take(self.results_shown_anim)
        {
            res.render(Vec2(x, y));
            y -= res.height() + SPACING;
        }

        if self.results_shown_anim == self.player_results.len() + 1 {
            self.winner_text.render_hcenter(r.width() as f32, SPACING);
        }

        r.present();
    }
}

impl StackableState for GameResultsState {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back | MenuButton::Start => {
                if self.exit_timer.is_none() {
                    self.exit_timer = Some(1.0)
                }
            }
            _ => {}
        }
        StackableStateResult::Continue
    }

    fn receive_return(&mut self, _retval: Box<dyn std::any::Any>) -> anyhow::Result<()> {
        panic!("Game result screen did not expect a return");
    }

    fn resize_screen(&mut self) {
        self.starfield
            .update_screensize(self.renderer.borrow().size());
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        if self.rounds_shown_anim < self.round_winners.len() && self.timer > 0.05 {
            self.rounds_shown_anim += 1;
            self.timer = 0.0;
        } else if self.results_shown_anim <= self.player_results.len() && self.timer > 0.5 {
            self.results_shown_anim += 1;
            self.timer = 0.0;
        } else {
            self.timer += timestep;
        }

        if let Some(mut exit) = self.exit_timer {
            exit -= timestep;
            if exit <= 0.0 {
                return StackableStateResult::Return(Box::new(self.starfield.clone()));
            }
            self.exit_timer = Some(exit);

            let alpha = exit;

            self.gameover_text.set_alpha(alpha);
            self.winner_text.set_alpha(alpha);
            for t in self.player_numbers.iter_mut() {
                t.set_alpha(alpha);
            }
            for t in self.player_results.iter_mut() {
                t.2.set_alpha(alpha);
            }
        }

        self.render();
        StackableStateResult::Continue
    }
}
