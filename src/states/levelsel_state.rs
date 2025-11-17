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
use log::warn;

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
    round_text: Text,
    timer: f32,
    selection: usize,
    columns: usize,
    renderer: Rc<RefCell<Renderer>>,
}

struct LevelBox {
    title: Text,
    thumbnail: Option<Texture>,
    target_rect: RectF,
    rect: RectF,
}

impl LevelSelection {
    const TOP_MARGIN: f32 = 64.0;
    const BOTTOM_MARGIN: f32 = 128.0;
    const BOX_SIZE: f32 = 256.0 + 6.0;

    const SELECTION_COLOR: Color = Color::new(0.328, 0.371, 0.496);

    pub fn new(
        assets: Rc<GameAssets>,
        round: i32,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let level_count = assets.levels.len();

        let round_text = renderer
            .borrow()
            .fontset()
            .menu_big
            .create_text(&renderer.borrow(), &format!("Round {}", round))?
            .with_color(Color::new(0.9, 0.2, 0.2));

        Ok(Self {
            assets,
            round_text,
            starfield,
            levelboxes: Vec::with_capacity(level_count),
            timer: 0.0,
            selection: 0,
            columns: 1,
            renderer,
        })
    }

    fn load_another_level(&mut self) {
        let renderer = &self.renderer.borrow();
        let level = &self.assets.levels[self.levelboxes.len()];
        self.levelboxes.push({
            let thumbnail = match Texture::from_file(renderer, level.thumbnail_path()) {
                Ok(t) => Some(t),
                Err(err) => {
                    warn!(
                        "Couldn't load thumbnail {:?}: {}",
                        level.thumbnail_path(),
                        err
                    );
                    None
                }
            };

            LevelBox {
                thumbnail,
                title: renderer
                    .fontset()
                    .menu
                    .create_text(renderer, level.title())
                    .unwrap()
                    .with_outline_color(Color::new(0.2, 0.2, 0.4)),
                target_rect: RectF::new(0.0, 0.0, 0.0, 0.0),
                rect: RectF::new(0.0, 0.0, 0.0, 0.0),
            }
        });
    }

    fn render(&self) {
        let renderer = &self.renderer.borrow();
        renderer.clear();

        // Render background
        self.starfield.borrow().render(renderer);

        // Round number
        self.round_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(renderer.width() as f32 / 2.0, 10.0)),
            ..Default::default()
        });

        // Render selection outline
        if let Some(lb) = self.levelboxes.get(self.selection) {
            renderer.draw_filled_rectangle(
                RectF::new(
                    lb.rect.x() - 3.0,
                    lb.rect.y() - 3.0,
                    lb.rect.w() + 6.0,
                    lb.rect.h() + 6.0,
                ),
                &Self::SELECTION_COLOR,
            );
        }

        // Render level selection boxes
        let bottom_edge = renderer.height() as f32 - Self::BOTTOM_MARGIN;
        for lb in self.levelboxes.iter() {
            let a = if lb.rect.y() < 0.0 {
                1.0 - -lb.rect.y().min(lb.rect.h()) / lb.rect.h()
            } else if lb.rect.bottom() > bottom_edge {
                1.0 - (lb.rect.bottom() - bottom_edge).min(lb.rect.h()) / lb.rect.h()
            } else {
                1.0
            };

            let border = Color::new(0.179, 0.195, 0.241);
            renderer.draw_filled_rectangle(lb.rect, &border);
            if let Some(tex) = &lb.thumbnail {
                tex.render(
                    renderer,
                    &RenderOptions {
                        dest: RenderDest::Centered(lb.rect.center()),
                        color: Color::WHITE.with_alpha(a),
                        ..Default::default()
                    },
                );
            }
        }

        // Selection title
        if let Some(lb) = self.levelboxes.get(self.selection) {
            lb.title.render(&RenderTextOptions {
                dest: RenderTextDest::BottomLeft(Vec2(10.0, renderer.height() as f32)),
                outline: TextOutline::Shadow,
                ..Default::default()
            });
        }

        renderer.present();
    }

    fn level_box_rects(count: usize, w: f32, h: f32) -> (Vec<RectF>, usize) {
        let mut rects = Vec::with_capacity(count);
        if count == 0 {
            return (rects, 1);
        }

        const SPACING: f32 = 32.0;

        let columns = ((w / (Self::BOX_SIZE + SPACING)).floor() as usize).min(count);
        let rows = count.div_ceil(columns);

        let left = (w - columns as f32 * (Self::BOX_SIZE + SPACING)) / 2.0;
        let top = Self::TOP_MARGIN.max((h - rows as f32 * (Self::BOX_SIZE + SPACING)) / 2.0);

        for row in 0..rows {
            let cols = columns.min(count - row * columns);
            for col in 0..cols {
                rects.push(RectF::new(
                    left + col as f32 * (Self::BOX_SIZE + SPACING),
                    top + row as f32 * (Self::BOX_SIZE + SPACING),
                    Self::BOX_SIZE,
                    Self::BOX_SIZE,
                ))
            }
        }

        (rects, columns)
    }

    fn update_levelbox_rects(&mut self) {
        let renderer = &self.renderer.borrow();
        let (rects, columns) = Self::level_box_rects(
            self.levelboxes.len(),
            renderer.width() as f32,
            renderer.height() as f32,
        );
        self.columns = columns;

        let scroll_offset = (self.selection / self.columns) as f32 * (Self::BOX_SIZE + 32.0);

        self.levelboxes
            .iter_mut()
            .zip(rects)
            .for_each(|(lb, r)| lb.target_rect = r.offset(0.0, -scroll_offset));
    }
}

impl StackableState for LevelSelection {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        let old_selection = self.selection;
        match button {
            MenuButton::Right(_) => {
                self.selection = (self.selection + 1) % self.levelboxes.len();
            }
            MenuButton::Left(_) => {
                self.selection = if self.selection > 0 {
                    self.selection - 1
                } else {
                    self.levelboxes.len() - 1
                };
            }
            MenuButton::Up(_) => {
                self.selection = if self.selection >= self.columns {
                    self.selection - self.columns
                } else {
                    (self.levelboxes.len() - (self.levelboxes.len() % self.columns)
                        + self.selection)
                        .min(self.levelboxes.len() - 1)
                }
            }
            MenuButton::Down(_) => {
                if self.selection + self.columns < self.levelboxes.len() {
                    self.selection += self.columns;
                } else {
                    self.selection %= self.columns;
                }
            }
            MenuButton::Back => {
                return StackableStateResult::Pop;
            }
            MenuButton::Start | MenuButton::Select(_) => {
                return StackableStateResult::Return(Box::new(
                    self.assets.levels[self.selection].clone(),
                ));
            }
            _ => {}
        }

        if self.selection != old_selection {
            self.update_levelbox_rects();
        }
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());

        self.update_levelbox_rects();
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        if self.levelboxes.len() < self.assets.levels.len() && self.timer <= 0.0 {
            self.load_another_level();
            self.update_levelbox_rects();
            if let Some(last) = self.levelboxes.last_mut() {
                last.rect = last.target_rect;
            }
            self.timer = 0.05;
        } else {
            self.timer -= timestep;
        }

        // Animate background
        self.starfield.borrow_mut().step(timestep);

        // Animate level boxes
        for lb in self.levelboxes.iter_mut() {
            let d = (lb.target_rect.topleft() - lb.rect.topleft()) * (10.0 * timestep);
            lb.rect = lb.rect.offset(d.0, d.1);
        }

        self.render();

        StackableStateResult::Continue
    }
}
