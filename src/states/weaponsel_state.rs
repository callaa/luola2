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
    demos::AnimatedStarfield,
    game::{GameControllerSet, MappedKey, MenuButton, Player, PlayerId, level::LEVEL_SCALE},
    gfx::{
        Color, RenderDest, RenderMode, RenderOptions, RenderTextDest, RenderTextOptions, Renderer,
        Text, TextOutline, Texture, make_button_icon,
    },
    math::{RectF, Vec2},
    states::game_assets::GameAssets,
};

struct Texts {
    menu: Text,
    flavor_title: Text,
    flavor_text: Text,
}

pub struct WeaponSelection {
    starfield: Rc<RefCell<AnimatedStarfield>>,
    assets: Rc<GameAssets>,
    background: Option<Texture>,
    background_rect: RectF,
    background_scroll: Vec2,
    round_text: Text,
    renderer: Rc<RefCell<Renderer>>,
    players: Vec<PlayerWeaponChoice>,
    texts: Vec<Texts>,
    longest_weapon_text_width: f32,
    flavortext_selection: usize,
    fadein: f32,
    background_fadein: f32,
    start_timer: Option<f32>,
}

struct PlayerWeaponChoice {
    controller: usize,
    selection: usize,
    ship_selection: usize,
    up_button_icon: Texture,
    down_button_icon: Texture,
    left_button_icon: Texture,
    right_button_icon: Texture,
    select_button_icon: Texture,
    decided: bool,
}

pub struct SelectedWeapons(pub Vec<(String, String)>);

impl PlayerWeaponChoice {
    fn from_weapon_name(
        player: &Player,
        assets: &GameAssets,
        renderer: &Renderer,
        controllers: &GameControllerSet,
    ) -> Result<Self> {
        let selection = assets
            .weapons
            .iter()
            .position(|w| w.name == player.weapon)
            .unwrap_or(
                assets
                    .weapons
                    .iter()
                    .position(|s| s.name == assets.default_weapon)
                    .expect("A default weapon should have been set in luola_main.lua"),
            );

        let ship_selection = assets
            .ships
            .iter()
            .position(|s| s.name == player.ship)
            .unwrap_or(
                assets
                    .ships
                    .iter()
                    .position(|s| s.name == assets.default_ship)
                    .expect("A default ship should have been set in luola_main.lua"),
            );

        Ok(Self {
            controller: player.controller as usize - 1,
            ship_selection,
            selection,
            up_button_icon: make_button_icon(
                player.controller,
                MappedKey::Up,
                renderer,
                controllers,
            )?,
            down_button_icon: make_button_icon(
                player.controller,
                MappedKey::Down,
                renderer,
                controllers,
            )?,
            left_button_icon: make_button_icon(
                player.controller,
                MappedKey::Left,
                renderer,
                controllers,
            )?,
            right_button_icon: make_button_icon(
                player.controller,
                MappedKey::Right,
                renderer,
                controllers,
            )?,
            select_button_icon: make_button_icon(
                player.controller,
                MappedKey::Fire1,
                renderer,
                controllers,
            )?,
            decided: false,
        })
    }
}

impl WeaponSelection {
    pub fn new(
        assets: Rc<GameAssets>,
        players: &[Player],
        round: i32,
        background: Option<Texture>,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        renderer: Rc<RefCell<Renderer>>,
        controllers: &GameControllerSet,
    ) -> Result<Self> {
        let round_text = renderer
            .borrow()
            .fontset()
            .menu_big
            .create_text(&renderer.borrow(), &format!("Round {}", round))?
            .with_color(Color::new(0.9, 0.2, 0.2));

        let flavortext_max_width = Self::flavortext_max_width(renderer.borrow().width());

        let texts = assets
            .weapons
            .iter()
            .map(|w| {
                let r = renderer.borrow();
                Ok(Texts {
                    menu: r.fontset().menu.create_text(&r, &w.title)?,
                    flavor_title: r
                        .fontset()
                        .flavotext
                        .create_text(&r, &w.title)?
                        .with_color(Color::new(1.0, 1.0, 0.8)),
                    flavor_text: r
                        .fontset()
                        .flavotext
                        .create_text(&r, &w.flavortext)?
                        .with_color(Color::new(0.9, 0.9, 0.9))
                        .with_wrapwidth(flavortext_max_width),
                })
            })
            .chain(assets.ships.iter().map(|s| {
                let r = renderer.borrow();
                Ok(Texts {
                    menu: r.fontset().menu.create_text(&r, &s.title)?,
                    flavor_title: r
                        .fontset()
                        .flavotext
                        .create_text(&r, &s.title)?
                        .with_color(Color::new(1.0, 0.9, 0.9)),
                    flavor_text: r
                        .fontset()
                        .flavotext
                        .create_text(&r, &s.flavortext)?
                        .with_color(Color::new(0.9, 0.9, 0.9))
                        .with_wrapwidth(flavortext_max_width),
                })
            }))
            .collect::<Result<Vec<_>>>()?;

        let longest_weapon_text_width = texts
            .iter()
            .fold(0.0, |acc, t| f32::max(acc, t.menu.width()));

        let choices = players
            .iter()
            .map(|player| {
                PlayerWeaponChoice::from_weapon_name(
                    player,
                    &assets,
                    &renderer.borrow(),
                    controllers,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let background_rect = if let Some(bg) = &background {
            Self::make_background_rect(bg, &renderer.borrow())
        } else {
            RectF::new(0.0, 0.0, 0.0, 0.0)
        };

        let flavortext_selection = choices[0].selection;
        Ok(Self {
            assets,
            background,
            background_rect,
            background_scroll: Vec2(15.0, 8.0),
            texts,
            players: choices,
            round_text,
            starfield,
            renderer,
            longest_weapon_text_width,
            flavortext_selection,
            start_timer: None,
            fadein: 0.0,
            background_fadein: 0.0,
        })
    }

    fn make_background_rect(tex: &Texture, renderer: &Renderer) -> RectF {
        let ar = renderer.height() as f32 / renderer.width() as f32;
        RectF::new(
            0.0,
            0.0,
            tex.width() / LEVEL_SCALE,
            tex.width() / LEVEL_SCALE * ar,
        )
    }

    fn flavortext_max_width(screen_width: i32) -> i32 {
        screen_width / 4
    }

    fn find_player_mut(&mut self, controller: i32) -> Option<&mut PlayerWeaponChoice> {
        assert!(controller > 0);
        let controller = (controller - 1) as usize;
        self.players.iter_mut().find(|p| p.controller == controller)
    }

    fn render_player_box(&self, player_id: PlayerId, player: &PlayerWeaponChoice, rect: RectF) {
        let renderer = &self.renderer.borrow();

        // Selected ship
        let mut x = rect.x() + 8.0;
        let icon_w = player.left_button_icon.width();
        let icon_h = player.left_button_icon.height();
        let text_h = self.texts[0].menu.height();

        let white = Color::WHITE.with_alpha(self.fadein);

        if !player.decided {
            player.up_button_icon.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(Vec2(
                        x + icon_w / 2.0,
                        rect.y() + (text_h - icon_h) / 2.0,
                    )),
                    color: white,
                    ..Default::default()
                },
            );
            player.down_button_icon.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(Vec2(
                        x + icon_w / 2.0,
                        rect.y() + (text_h + icon_h) / 2.0,
                    )),
                    color: white,
                    ..Default::default()
                },
            );

            x += icon_w + 8.0;
        }

        let shiptexid = self.assets.ships[player.ship_selection].texture;
        let shiptex = renderer.texture_store().get_texture(shiptexid);
        let mut ship_render = RenderOptions {
            dest: RenderDest::Centered(Vec2(x + shiptex.width() / 2.0, rect.y() + text_h / 2.0)),
            mode: RenderMode::Rotated(if player.decided { 0.0 } else { 90.0 }, false),
            color: white,

            ..Default::default()
        };

        shiptex.render(renderer, &ship_render);
        ship_render.color = Color::player_color(player_id).with_alpha(self.fadein);
        renderer
            .texture_store()
            .get_texture_alt(shiptexid, crate::gfx::TexAlt::Decal)
            .expect("Ships should have a Decal alt-texture")
            .render(renderer, &ship_render);

        x += shiptex.width() + 8.0;

        // Selected weapon
        let title_text = &self.texts[player.selection].menu;

        if !player.decided {
            player.left_button_icon.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(Vec2(x + icon_w / 2.0, rect.y() + icon_h / 2.0)),
                    color: white,
                    ..Default::default()
                },
            );

            x += icon_w + 8.0;
        }

        title_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopLeft(Vec2(x, rect.y())),
            outline: TextOutline::Outline,
            alpha: self.fadein,
            ..Default::default()
        });

        x += title_text.width() + 8.0;

        if !player.decided {
            player.right_button_icon.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(Vec2(x + icon_w / 2.0, rect.y() + icon_h / 2.0)),
                    color: white,
                    ..Default::default()
                },
            );

            x += icon_w + 16.0;

            player.select_button_icon.render(
                renderer,
                &RenderOptions {
                    dest: RenderDest::Centered(Vec2(x + icon_w / 2.0, rect.y() + icon_h / 2.0)),
                    color: white,
                    ..Default::default()
                },
            );
        }
    }

    fn render(&self) {
        let renderer = &self.renderer.borrow();
        renderer.clear();

        // Render background
        self.starfield.borrow().render(renderer);
        if let Some(bg) = &self.background {
            bg.render(
                renderer,
                &RenderOptions {
                    source: Some(self.background_rect),
                    dest: RenderDest::Fill,
                    color: Color::WHITE.with_alpha(self.background_fadein),
                    ..Default::default()
                },
            );
        }

        // Round number (shared with previous state so not faded in)
        self.round_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(Vec2(renderer.width() as f32 / 2.0, 10.0)),
            outline: TextOutline::Outline,
            ..Default::default()
        });

        // Player weapon selection boxes
        let texts = &self.texts[self.flavortext_selection];
        let flavortext_box_w = texts.flavor_title.width().max(texts.flavor_text.width());
        let flavortext_box_x = renderer.width() as f32 - flavortext_box_w - 32.0;

        let player_box_w = 90.0 + self.longest_weapon_text_width;
        let player_box_h = self.texts[0]
            .menu
            .height()
            .max(self.players[0].up_button_icon.height() * 2.0)
            + 16.0;

        let mut player_box = RectF::new(
            (renderer.width() as f32 - player_box_w) / 2.0,
            (renderer.height() as f32 - player_box_h * self.players.len() as f32) / 2.0,
            player_box_w,
            player_box_h,
        );

        if player_box.right() > flavortext_box_x {
            player_box = player_box + Vec2(flavortext_box_x - player_box.right(), 0.0);
        }

        for (idx, player) in self.players.iter().enumerate() {
            self.render_player_box(idx as PlayerId + 1, player, player_box);
            player_box = player_box + Vec2(0.0, player_box_h);
        }

        // Weapon flavor text box
        let flavortext_box_h = texts.flavor_title.height() + texts.flavor_text.height();
        let hint_box = RectF::new(
            flavortext_box_x,
            renderer.height() as f32 - flavortext_box_h - 32.0,
            flavortext_box_w + 16.0,
            flavortext_box_h + 24.0,
        );

        renderer.draw_filled_rectangle(hint_box, &Color::new_rgba(0.1, 0.1, 0.2, 0.9));
        texts.flavor_title.render(&RenderTextOptions {
            dest: RenderTextDest::TopCenter(hint_box.topleft() + Vec2(hint_box.w() / 2.0, 8.0)),
            ..Default::default()
        });
        texts.flavor_text.render(&RenderTextOptions {
            dest: RenderTextDest::TopLeft(
                hint_box.topleft() + Vec2(8.0, texts.flavor_title.height() + 16.0),
            ),
            ..Default::default()
        });

        // Fadeout when it's time to start
        if let Some(t) = self.start_timer {
            let a = 1.0 - t;
            renderer.draw_filled_rectangle(
                RectF::new(0.0, 0.0, renderer.width() as f32, renderer.height() as f32),
                &Color::new_rgba(0.0, 0.0, 0.0, a),
            );
        }

        renderer.present();
    }
}

impl StackableState for WeaponSelection {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        let weapon_count = self.assets.weapons.len();
        let ship_count = self.assets.ships.len();

        match button {
            MenuButton::Back => {
                return StackableStateResult::Pop;
            }
            MenuButton::Up(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr)
                    && !p.decided
                {
                    if p.ship_selection > 0 {
                        p.ship_selection -= 1;
                    } else {
                        p.ship_selection = ship_count - 1;
                    }
                    self.flavortext_selection = weapon_count + p.ship_selection;
                }
            }
            MenuButton::Down(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr)
                    && !p.decided
                {
                    p.ship_selection = (p.ship_selection + 1) % ship_count;
                    self.flavortext_selection = weapon_count + p.ship_selection;
                }
            }
            MenuButton::Left(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr)
                    && !p.decided
                {
                    if p.selection > 0 {
                        p.selection -= 1;
                    } else {
                        p.selection = weapon_count - 1;
                    }
                    self.flavortext_selection = p.selection;
                }
            }
            MenuButton::Right(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr)
                    && !p.decided
                {
                    p.selection = (p.selection + 1) % weapon_count;
                    self.flavortext_selection = p.selection;
                }
            }
            MenuButton::Select(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr) {
                    p.decided = !p.decided;
                }

                if self.players.iter().all(|p| p.decided) {
                    self.start_timer = Some(1.0);
                }
            }
            _ => {}
        }

        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());

        let ww = Self::flavortext_max_width(self.renderer.borrow().width());
        self.texts
            .iter_mut()
            .for_each(|t| t.flavor_text.set_wrapwidth(ww));

        if let Some(bg) = &self.background {
            self.background_rect = Self::make_background_rect(bg, &self.renderer.borrow());
        }
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        // Animate background
        self.starfield.borrow_mut().step(timestep);

        self.fadein = (self.fadein + timestep).min(1.0);
        self.background_fadein = (self.background_fadein + timestep * 0.3).min(1.0);

        if let Some(bg) = &self.background {
            self.background_rect = self.background_rect + self.background_scroll * timestep;
            if self.background_rect.right() >= bg.width() || self.background_rect.x() <= 0.0 {
                self.background_scroll =
                    self.background_scroll.element_wise_product(Vec2(-1.0, 1.0));
            }

            if self.background_rect.bottom() >= bg.height() || self.background_rect.y() <= 0.0 {
                self.background_scroll =
                    self.background_scroll.element_wise_product(Vec2(1.0, -1.0));
            }
        }

        // Game start fadeout animation
        if let Some(t) = self.start_timer {
            if self.players.iter().any(|p| !p.decided) {
                self.start_timer = None;
            } else {
                let t = t - timestep;
                if t <= 0.0 {
                    return StackableStateResult::Return(Box::new(SelectedWeapons(
                        self.players
                            .iter()
                            .map(|p| {
                                (
                                    self.assets.ships[p.ship_selection].name.clone(),
                                    self.assets.weapons[p.selection].name.clone(),
                                )
                            })
                            .collect(),
                    )));
                }
                self.start_timer = Some(t);
            }
        }

        self.render();

        StackableStateResult::Continue
    }
}
