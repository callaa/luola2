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

use super::{StackableState, StackableStateResult};
use crate::{
    demos::AnimatedStarfield,
    game::{GameControllerSet, MenuButton, Player},
    gfx::{
        Color, RenderDest, RenderOptions, RenderTextDest, RenderTextOptions, Renderer, Text,
        TextOutline, Texture, make_controller_icon,
    },
    math::{RectF, Vec2},
    states::{GameState, game_assets::GameAssets},
};

/**
 * Player selection state prepares the game state
 */
pub struct PlayerSelection {
    assets: Rc<GameAssets>,
    starfield: Rc<RefCell<AnimatedStarfield>>,
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,

    start_text: Text,
    prompt_text: Text,
    rounds_text: Text,

    rounds_to_win: i32,
    rounds_to_win_text: Text,
    players: Vec<JoiningPlayer>,

    /// Fade out timer after which the game will start
    start_timer: Option<f32>,
}

struct JoiningPlayer {
    controller: i32,
    join_button_pressed: bool,
    target_rect: RectF,
    rect: RectF,
    icon: Texture,
    text: Text,
}

impl PlayerSelection {
    pub fn new(
        assets: Rc<GameAssets>,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Self {
        let r = renderer.borrow();
        let font = &r.fontset().menu;
        let red = Color::new(0.9, 0.2, 0.2);

        let prompt_text = font
            .create_text(&r, "Press Fire to join!")
            .unwrap()
            .with_outline_color(Color::new(0.2, 0.2, 0.4));
        let rounds_text = r
            .fontset()
            .menu
            .create_text(&r, "ROUNDS")
            .unwrap()
            .with_color(red);
        let start_text = font
            .create_text(&r, "Press Enter to start the game!")
            .unwrap()
            //.with_color(red);
            .with_outline_color(Color::new(0.2, 0.2, 0.4));

        let rounds_to_win = 5;
        let rounds_to_win_text = r
            .fontset()
            .menu_big
            .create_text(&r, &format!("{:02}", rounds_to_win))
            .unwrap()
            .with_color(Color::new(0.9, 0.2, 0.2));
        drop(r);

        Self {
            assets,
            starfield,
            renderer,
            controllers,
            prompt_text,
            rounds_text,
            start_text,
            rounds_to_win,
            rounds_to_win_text,
            players: Vec::new(),
            start_timer: None,
        }
    }

    pub fn render(&self) {
        let renderer = self.renderer.borrow();
        renderer.clear();
        let w = renderer.width() as f32;
        let h = renderer.height() as f32;

        let fadeout = self.start_timer.unwrap_or(0.0);

        // Background
        self.starfield.borrow().render(&renderer);

        // Player list
        if self.players.is_empty() {
            self.prompt_text.render(&RenderTextOptions {
                dest: RenderTextDest::Centered(Vec2(w / 2.0, h / 2.0)),
                outline: TextOutline::Shadow,
                ..Default::default()
            });
        } else {
            for p in &self.players {
                let rect = p.rect.offset(fadeout.powf(2.0) * (w - p.rect.x()), 0.0);

                p.icon.render(
                    &renderer,
                    &RenderOptions {
                        dest: RenderDest::Centered(rect.center()),
                        ..Default::default()
                    },
                );

                p.text.render(&RenderTextOptions {
                    dest: RenderTextDest::BottomCenter(Vec2(
                        rect.x() + rect.w() / 2.0,
                        rect.bottom(),
                    )),
                    ..Default::default()
                });
            }
        }

        // Round selector
        let offset_x = fadeout.powf(2.0) * w / 2.0;
        let offset_y = 10.0;

        self.rounds_to_win_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(
                w / 2.0 - offset_x,
                //h - self.rounds_text.height() - offset_y,
                offset_y,
            )),
            ..Default::default()
        });

        self.rounds_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(
                w / 2.0 - offset_x,
                offset_y + self.rounds_to_win_text.height(),
            )),
            ..Default::default()
        });

        // Start game prompt
        if !self.players.is_empty() {
            self.start_text.render(&RenderTextOptions {
                dest: RenderTextDest::BottomCenter(Vec2(w / 2.0 - offset_x, h - offset_y)),
                outline: TextOutline::Shadow,
                ..Default::default()
            });
        }
        renderer.present();
    }

    fn player_box_rects(player_count: usize, renderer: &Renderer) -> Vec<RectF> {
        let size = 160.0;
        let mut rects = Vec::with_capacity(player_count);
        if player_count == 0 {
            return rects;
        }

        const SPACING: f32 = 32.0;

        let columns =
            ((renderer.width() as f32 / (size + SPACING)).floor() as usize).min(player_count);
        let rows = player_count.div_ceil(columns);

        let left = (renderer.width() as f32 - columns as f32 * (size + SPACING)) / 2.0;
        let top = (renderer.height() as f32 - rows as f32 * (size + SPACING)) / 2.0;

        for row in 0..rows {
            let cols = columns.min(player_count - row * columns);
            for col in 0..cols {
                rects.push(RectF::new(
                    left + col as f32 * (size + SPACING),
                    top + row as f32 * (size + SPACING),
                    size,
                    size,
                ))
            }
        }

        rects
    }
}

impl StackableState for PlayerSelection {
    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());

        Self::player_box_rects(self.players.len(), &self.renderer.borrow())
            .iter()
            .zip(self.players.iter_mut())
            .for_each(|(rect, p)| p.target_rect = *rect);
    }

    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back => {
                return StackableStateResult::Pop;
            }
            MenuButton::Left(_) => {
                if self.rounds_to_win > 1 {
                    self.rounds_to_win -= 1;
                    self.rounds_to_win_text
                        .set_text(&format!("{:02}", self.rounds_to_win));
                }
            }
            MenuButton::Right(_) => {
                if self.rounds_to_win < 99 {
                    self.rounds_to_win += 1;
                    self.rounds_to_win_text
                        .set_text(&format!("{:02}", self.rounds_to_win));
                }
            }
            MenuButton::Start => {
                if !self.players.is_empty() {
                    self.start_timer = Some(0.0);
                }
            }
            MenuButton::Select(controller) if controller > 0 => {
                let player_idx = self
                    .players
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.controller == controller)
                    .map(|(idx, _)| idx);

                if let Some(player_idx) = player_idx {
                    // Remove a player
                    self.players.remove(player_idx);
                    Self::player_box_rects(self.players.len(), &self.renderer.borrow())
                        .iter()
                        .zip(self.players.iter_mut())
                        .enumerate()
                        .for_each(|(idx, (rect, p))| {
                            p.target_rect = *rect;
                            p.text.set_text(&format!("P{}", idx + 1));
                            p.text
                                .set_default_color(Color::player_color(idx as i32 + 1));
                        });
                } else {
                    // Add a player
                    self.players.push(JoiningPlayer {
                        controller,
                        join_button_pressed: true,
                        rect: RectF::new(0.0, 0.0, 0.0, 0.0),
                        target_rect: RectF::new(0.0, 0.0, 0.0, 0.0),
                        text: self
                            .renderer
                            .borrow()
                            .fontset()
                            .menu
                            .create_text(
                                &self.renderer.borrow(),
                                &format!("P{}", self.players.len() + 1),
                            )
                            .unwrap()
                            .with_color(Color::player_color(self.players.len() as i32 + 1)),
                        icon: match make_controller_icon(
                            controller,
                            &self.renderer.borrow(),
                            &self.controllers.borrow(),
                        ) {
                            Ok(icon) => icon,
                            Err(err) => return StackableStateResult::Error(err),
                        },
                    });

                    let boxes = Self::player_box_rects(self.players.len(), &self.renderer.borrow());
                    for (player, rect) in self.players.iter_mut().zip(boxes.iter()) {
                        player.target_rect = *rect;
                    }
                    self.players.last_mut().unwrap().rect = *boxes.last().unwrap();
                }
            }
            _ => {}
        }
        StackableStateResult::Continue
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        self.starfield.borrow_mut().step(timestep);

        // Animate player boxes
        for p in self.players.iter_mut() {
            let d = (p.target_rect.topleft() - p.rect.topleft()) * (10.0 * timestep);
            p.rect = p.rect.offset(d.0, d.1);
        }

        // Fadeout then start game
        if let Some(mut start) = self.start_timer {
            start += timestep * 3.0;
            if start > 1.0 {
                let players: Vec<Player> = self
                    .players
                    .iter()
                    .map(|p| Player::new(p.controller))
                    .collect();

                for (idx, plr) in players.iter().enumerate() {
                    self.controllers
                        .borrow()
                        .set_player_leds(plr.controller, idx as i32 + 1);
                }

                return StackableStateResult::Replace(Box::new(GameState::new(
                    self.assets.clone(),
                    players,
                    self.rounds_to_win,
                    self.starfield.clone(),
                    self.controllers.clone(),
                    self.renderer.clone(),
                )));
            } else {
                self.start_timer = Some(start);
            }
        }

        self.render();
        StackableStateResult::Continue
    }
}
