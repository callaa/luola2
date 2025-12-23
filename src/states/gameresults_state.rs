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
    gfx::{Color, RenderTextDest, RenderTextOptions, Renderer, Text, TextOutline},
    math::{Vec2, interpolation},
    menu::AnimatedStarfield,
    states::{StackableState, StackableStateResult},
};

enum AnimationState {
    // Initial game-over text fade-in
    FadeIn(f32),

    // Text scrolls up, reveals ranking table
    Scroll(f32),

    // Reveal round results list
    RoundResults(f32),

    // Wait for any key
    Wait,

    // Fade out
    Exit(f32),
}

pub struct GameResultsState {
    players: Vec<Player>,
    round_winners: Vec<PlayerId>,
    renderer: Rc<RefCell<Renderer>>,

    starfield: AnimatedStarfield,
    gameover_text: Text,
    player_numbers: Vec<Text>,
    player_ranking: Vec<(i32, i32, Text)>,
    ranking_table_size: (f32, f32),
    anim: AnimationState,
}

impl GameResultsState {
    pub fn new(
        players: Vec<Player>,
        round_winners: Vec<PlayerId>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let r = renderer.borrow();

        // New static background which will be passed to the main menu state
        let starfield = AnimatedStarfield::new(200, r.width() as f32, r.height() as f32);

        // Player numbers used to indicate round winners. Player 0 means tie
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

        // Player ranking table
        let mut player_ranking = players
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

        player_ranking.sort_by(|a, b| a.0.cmp(&b.0));

        let ranking_table_size = player_ranking
            .iter()
            .map(|(_, _, txt)| (txt.width(), txt.height()))
            .reduce(|acc, (w, h)| (acc.0.max(w), acc.1 + h))
            .expect("There should be at least one player");

        // The headline
        let gameover_text = r
            .fontset()
            .menu_big
            .create_text(&r, "Game Over!")?
            .with_outline_color(Color::new(0.2, 0.2, 0.4));

        drop(r);

        Ok(Self {
            players,
            round_winners,
            renderer,
            starfield,
            gameover_text,
            player_numbers,
            player_ranking,
            ranking_table_size,
            anim: AnimationState::FadeIn(0.0),
        })
    }

    fn render(&self) {
        let r = self.renderer.borrow();

        r.clear();

        let w = r.width() as f32;
        let h = r.height() as f32;

        let alpha = match self.anim {
            AnimationState::FadeIn(t) => t,
            AnimationState::Exit(t) => 1.0 - t,
            _ => 1.0,
        };

        // Starfield does not fade out as its shared with the main menu this state returns to
        self.starfield.render_with_alpha(
            &r,
            match self.anim {
                AnimationState::FadeIn(t) => t,
                _ => 1.0,
            },
        );

        const SPACING: f32 = 5.0;

        // Heading
        let heading_y = match self.anim {
            AnimationState::FadeIn(_) => h / 2.0,
            AnimationState::Scroll(t) => {
                interpolation::linear(h / 2.0, self.gameover_text.height() / 2.0 + SPACING, t)
            }
            _ => self.gameover_text.height() / 2.0 + SPACING,
        };

        self.gameover_text.render(&RenderTextOptions {
            dest: RenderTextDest::Centered(Vec2(r.width() as f32 / 2.0, heading_y)),
            outline: TextOutline::Shadow,
            alpha,
            ..Default::default()
        });

        // Round result table
        let rounds_shown = match self.anim {
            AnimationState::FadeIn(_) => 0,
            AnimationState::Scroll(_) => 0,
            AnimationState::RoundResults(t) => (self.round_winners.len() as f32 * t) as usize,
            _ => self.round_winners.len(),
        };

        if rounds_shown > 0 {
            let rounds_width =
                (self.player_numbers[0].width() + SPACING) * self.round_winners.len() as f32;

            let mut rounds_x = (w - rounds_width) / 2.0;
            let rounds_y = self.gameover_text.height() + SPACING * 3.0;

            for r in self.round_winners.iter().take(rounds_shown) {
                let text = &self.player_numbers[*r as usize];
                text.render(&RenderTextOptions {
                    dest: RenderTextDest::TopLeft(Vec2(rounds_x, rounds_y)),
                    alpha,
                    ..Default::default()
                });
                rounds_x += text.width() + SPACING;
            }
        }

        // Player ranking table
        let ranking_alpha = match self.anim {
            AnimationState::FadeIn(_) => 0.0,
            AnimationState::Scroll(t) => t,
            _ => alpha,
        };

        if ranking_alpha > 0.0 {
            let x = (w - self.ranking_table_size.0) / 2.0;
            let mut y = (h + self.ranking_table_size.1) / 2.0;

            let heading_y = heading_y + self.gameover_text.height();
            for (_, _, res) in self.player_ranking.iter() {
                if y > heading_y {
                    res.render(&RenderTextOptions {
                        dest: RenderTextDest::TopLeft(Vec2(x, y)),
                        alpha: ranking_alpha,
                        ..Default::default()
                    });

                    y -= res.height() + SPACING;
                }
            }
        }

        r.present();
    }
}

impl StackableState for GameResultsState {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back | MenuButton::Start | MenuButton::Select(_) => match self.anim {
                AnimationState::FadeIn(_)
                | AnimationState::Scroll(_)
                | AnimationState::RoundResults(_) => self.anim = AnimationState::Wait,
                AnimationState::Wait => self.anim = AnimationState::Exit(0.0),
                AnimationState::Exit(_) => return StackableStateResult::Pop,
            },
            _ => {}
        }
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        self.starfield
            .update_screensize(self.renderer.borrow().size());
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        self.anim = match self.anim {
            AnimationState::FadeIn(t) => {
                let t = t + timestep;
                if t > 1.0 {
                    AnimationState::Scroll(0.0)
                } else {
                    AnimationState::FadeIn(t)
                }
            }

            AnimationState::Scroll(t) => {
                let t = t + timestep;
                if t > 1.0 {
                    AnimationState::RoundResults(0.0)
                } else {
                    AnimationState::Scroll(t)
                }
            }

            AnimationState::RoundResults(t) => {
                let t = t + timestep;
                if t > 1.0 {
                    AnimationState::Wait
                } else {
                    AnimationState::RoundResults(t)
                }
            }

            AnimationState::Wait => AnimationState::Wait,

            AnimationState::Exit(t) => {
                let t = t + timestep;
                if t > 1.0 {
                    return StackableStateResult::Return(Box::new(self.starfield.clone()));
                } else {
                    AnimationState::Exit(t)
                }
            }
        };

        self.render();
        StackableStateResult::Continue
    }
}
