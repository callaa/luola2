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

use super::{StackableState, StackableStateResult};
use crate::{
    game::MenuButton,
    gfx::{
        Color, RenderDest, RenderOptions, RenderTextDest, RenderTextOptions, Renderer, Text,
        TextOutline, Texture,
    },
    math::{RectF, Vec2},
    menu::AnimatedStarfield,
    states::game_assets::GameAssets,
};

pub struct LevelSelection {
    starfield: Rc<RefCell<AnimatedStarfield>>,
    assets: Rc<GameAssets>,
    levelboxes: Vec<LevelBox>,
    prev_round_text: Option<Text>,
    fadein_round_text: bool,
    round_text: Text,
    selection: usize,
    selector_offset: f32,
    selector_offset_target: f32,
    renderer: Rc<RefCell<Renderer>>,
    fadein: f32,
    fadeout: f32,
    start: bool,
}

struct LevelBox {
    title: Text,
    thumbnail: Option<Texture>,
    w: f32,
    h: f32,
    xpos: f32,
}

impl LevelSelection {
    const TOP_MARGIN: f32 = 64.0;
    const BOTTOM_MARGIN: f32 = 128.0;
    const BOX_SIZE: f32 = 256.0 + 6.0;

    const SELECTION_COLOR: Color = Color::new(0.328, 0.371, 0.496);

    pub fn new(
        assets: Rc<GameAssets>,
        round: i32,
        fadein_round_text: bool,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        renderer: Rc<RefCell<Renderer>>,
        selection: usize,
    ) -> Result<Self> {
        debug_assert!(selection < assets.levels.len());
        let round_text = renderer
            .borrow()
            .fontset()
            .menu_big
            .create_text(&renderer.borrow(), &format!("Round {}", round))?
            .with_color(Color::new(0.9, 0.2, 0.2));

        let prev_round_text = if round > 1 {
            Some(
                renderer
                    .borrow()
                    .fontset()
                    .menu_big
                    .create_text(&renderer.borrow(), &format!("Round {}", round - 1))?
                    .with_color(Color::new(0.9, 0.2, 0.2)),
            )
        } else {
            None
        };

        let mut last_xpos = 0.0;

        let levelboxes: Vec<LevelBox> = assets
            .levels
            .iter()
            .map(|level| {
                let (w, h) = if let Some(t) = level.thumbnail() {
                    (t.width(), t.height())
                } else {
                    (512.0, 512.0)
                };

                let xpos = last_xpos + 40.0;
                last_xpos = xpos + w;
                LevelBox {
                    thumbnail: level.thumbnail().cloned(),
                    title: renderer
                        .borrow()
                        .fontset()
                        .menu
                        .create_text(&renderer.borrow(), level.title())
                        .unwrap()
                        .with_outline_color(Color::new(0.2, 0.2, 0.4)),
                    w,
                    h,
                    xpos,
                }
            })
            .collect();

        let selector_offset = levelboxes[selection].xpos + levelboxes[selection].w / 2.0;

        Ok(Self {
            assets,
            round_text,
            prev_round_text,
            fadein_round_text,
            starfield,
            levelboxes,
            selection,
            selector_offset,
            selector_offset_target: selector_offset,
            renderer,
            fadein: 0.0,
            fadeout: 1.0,
            start: false,
        })
    }

    fn render(&self) {
        let renderer = &self.renderer.borrow();
        renderer.clear();

        // Render background
        self.starfield.borrow().render(renderer);

        // Round number
        let round_fadein = if self.fadein < 1.0 && self.fadein_round_text {
            self.fadein
        } else {
            1.0
        };

        if round_fadein < 1.0
            && let Some(prev_round_text) = &self.prev_round_text
        {
            prev_round_text.render(&RenderTextOptions {
                dest: RenderTextDest::TopCenter(Vec2(
                    renderer.width() as f32 / 2.0,
                    10.0 - prev_round_text.height()
                        + prev_round_text.height() * (1.0 - round_fadein).powf(2.0),
                )),
                outline: TextOutline::Outline,
                alpha: 1.0 - round_fadein,
                ..Default::default()
            });
        }

        self.round_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(
                renderer.width() as f32 / 2.0,
                10.0 + if self.prev_round_text.is_some() {
                    self.round_text.height() * (1.0 - round_fadein).powf(2.0)
                } else {
                    0.0
                },
            )),
            outline: TextOutline::Outline,
            alpha: round_fadein,
            ..Default::default()
        });

        let center = Vec2(
            (renderer.width() / 2) as f32,
            (renderer.height() / 2) as f32,
        );

        // Level thumbnails
        let selected_level = &self.levelboxes[self.selection];
        let offset = center.0 - self.selector_offset; //selected_level.xpos - selected_level.w / 2.0;
        for (i, level) in self.levelboxes.iter().enumerate() {
            if let Some(t) = &level.thumbnail {
                let d = (self.selection as i32 - i as i32).abs() as f32;
                t.render(
                    renderer,
                    &RenderOptions {
                        dest: RenderDest::Rect(RectF::new(
                            level.xpos + offset,
                            center.1 - level.h / 2.0,
                            level.w,
                            level.h,
                        )),
                        color: if d > 0.0 {
                            Color::WHITE
                                .with_alpha(self.fadein * self.fadeout * (1.0 / (2.0 + d * d)))
                        } else {
                            Color::WHITE.with_alpha(self.fadein * self.fadeout)
                        },
                        ..Default::default()
                    },
                );
            }
        }

        // Selected level info
        let mut levelinfo_center = center + Vec2(0.0, 512.0 / 2.0 + 10.0);
        if self.fadeout < 1.0 {
            levelinfo_center = Vec2(
                levelinfo_center.0,
                levelinfo_center.1
                    + (renderer.height() as f32 - levelinfo_center.1)
                        * (1.0 - self.fadeout.powf(2.0)),
            );
        }
        selected_level.title.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(levelinfo_center),
            outline: TextOutline::Shadow,
            alpha: self.fadein,
            ..Default::default()
        });

        renderer.present();
    }
}

impl StackableState for LevelSelection {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Right(_) if !self.start => {
                self.selection = (self.selection + 1) % self.levelboxes.len();
            }
            MenuButton::Left(_) if !self.start => {
                self.selection =
                    (self.selection as i32 - 1).rem_euclid(self.levelboxes.len() as i32) as usize;
            }
            MenuButton::Back => {
                return StackableStateResult::Pop;
            }
            MenuButton::Start | MenuButton::Select(_) => {
                self.start = true;
            }
            _ => {}
        }

        let selected_level = &self.levelboxes[self.selection];
        self.selector_offset_target = selected_level.xpos + selected_level.w / 2.0;
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        // Animate background
        self.starfield.borrow_mut().step(timestep);

        // Fadein (note: background is shared with other states and does not need to fade in)
        if self.fadein < 1.0 {
            self.fadein += timestep;
        }

        // Animated transition to next state
        if self.start {
            self.fadeout -= timestep * 2.0;

            if self.fadeout <= 0.0 {
                return StackableStateResult::Return(Box::new(
                    self.assets.levels[self.selection].clone(),
                ));
            }
        }

        // Animate level boxes
        self.selector_offset +=
            (self.selector_offset_target - self.selector_offset) * timestep * 10.0;

        self.render();

        StackableStateResult::Continue
    }
}
