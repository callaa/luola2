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

use anyhow::{Result, anyhow};

use super::{StackableState, StackableStateResult};
use crate::{
    game::{GameControllerSet, MenuButton, Player, PlayerId},
    gfx::{Color, RenderMode, RenderOptions, Renderer, Text, Texture, TextureId, make_button_icon},
    math::{RectF, Vec2},
    menu::AnimatedStarfield,
    states::game_assets::{GameAssets, SelectableWeapon},
};

pub struct WeaponSelection {
    starfield: Rc<RefCell<AnimatedStarfield>>,
    assets: Rc<GameAssets>,
    box_border: TextureId,
    round_text: Text,
    renderer: Rc<RefCell<Renderer>>,
    players: Vec<PlayerWeaponChoice>,
    weapon_texts: Vec<(Text, Text)>,
    longest_weapon_text_width: f32,
    start_timer: Option<f32>,
}

struct PlayerWeaponChoice {
    controller: usize,
    selection: usize,
    player_text: Text,
    left_button_icon: Texture,
    right_button_icon: Texture,
    select_button_icon: Texture,
    decided: bool,
}

pub struct SelectedWeapons(pub Vec<String>);

impl PlayerWeaponChoice {
    fn from_weapon_name(
        player: &Player,
        player_id: PlayerId,
        weapon_list: &[SelectableWeapon],
        renderer: &Renderer,
        controllers: &GameControllerSet,
    ) -> Result<Self> {
        let selection = weapon_list
            .iter()
            .enumerate()
            .find(|(_, w)| w.name == player.weapon)
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        Ok(Self {
            controller: player.controller as usize - 1,
            selection,
            player_text: renderer
                .fontset()
                .menu
                .create_text(renderer, &format!("Player {}", player_id))?
                .with_color(Color::player_color(player_id)),
            left_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Left,
                renderer,
                controllers,
            )?,
            right_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Right,
                renderer,
                controllers,
            )?,
            select_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Fire1,
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

        let weapon_texts = assets
            .weapons
            .iter()
            .map(|w| {
                let r = renderer.borrow();
                Ok((
                    r.fontset().menu.create_text(&r, &w.title)?,
                    r.fontset()
                        .flavotext
                        .create_text(&r, &w.flavortext)?
                        .with_color(Color::new(0.6, 0.6, 0.8))
                        .with_wrapwidth(flavortext_max_width),
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let longest_weapon_text_width = weapon_texts
            .iter()
            .fold(0.0, |acc, t| f32::max(acc, t.0.width()));

        let choices: Result<Vec<_>> = players
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                PlayerWeaponChoice::from_weapon_name(
                    player,
                    idx as PlayerId + 1,
                    &assets.weapons,
                    &renderer.borrow(),
                    controllers,
                )
            })
            .collect();

        let box_border = renderer
            .borrow()
            .texture_store()
            .find_texture("box_border")?;

        Ok(Self {
            assets,
            box_border,
            weapon_texts,
            players: choices?,
            round_text,
            starfield,
            renderer,
            longest_weapon_text_width,
            start_timer: None,
        })
    }

    fn flavortext_max_width(screen_width: i32) -> i32 {
        screen_width * 3 / 4 - 16
    }

    fn find_player_mut(&mut self, controller: i32) -> Option<&mut PlayerWeaponChoice> {
        assert!(controller > 0);
        let controller = (controller - 1) as usize;
        self.players.iter_mut().find(|p| p.controller == controller)
    }

    fn render_player_box(&self, player: &PlayerWeaponChoice, rect: RectF) {
        let renderer = &self.renderer.borrow();

        player.player_text.render(Vec2(rect.x() + 8.0, rect.y()));

        let rect = RectF::new(
            rect.x(),
            rect.y() + player.player_text.height(),
            rect.w(),
            rect.h() - player.player_text.height(),
        );

        renderer
            .texture_store()
            .get_texture(self.box_border)
            .render(
                renderer,
                &RenderOptions {
                    dest: crate::gfx::RenderDest::Rect(rect),
                    mode: RenderMode::NineGrid(1.0),
                    ..Default::default()
                },
            );

        let (title_text, flavor_text) = &self.weapon_texts[player.selection];
        let weapon_title_x = rect.x() + (rect.w() - title_text.width()) / 2.0;

        if player.decided {
            title_text.render(Vec2(
                weapon_title_x,
                rect.y() + (rect.h() - title_text.height()) / 2.0,
            ));
        } else {
            let y = rect.y() + 8.0;
            title_text.render(Vec2(
                weapon_title_x,
                y + (title_text.height().max(player.left_button_icon.height())
                    - title_text.height())
                    / 2.0,
            ));

            flavor_text.render(Vec2(rect.x() + 8.0, y + title_text.height() + 16.0));

            player.left_button_icon.render_simple(
                renderer,
                None,
                Some(RectF::new(
                    weapon_title_x - player.left_button_icon.width() - 8.0,
                    y,
                    player.left_button_icon.width(),
                    player.left_button_icon.height(),
                )),
            );
            player.right_button_icon.render_simple(
                renderer,
                None,
                Some(RectF::new(
                    weapon_title_x + title_text.width() + 8.0,
                    y,
                    player.right_button_icon.width(),
                    player.right_button_icon.height(),
                )),
            );
            player.select_button_icon.render_simple(
                renderer,
                None,
                Some(RectF::new(
                    rect.x() + rect.w() - player.select_button_icon.width() - 8.0,
                    rect.y() + rect.h() - player.select_button_icon.height() - 8.0,
                    player.select_button_icon.width(),
                    player.select_button_icon.height(),
                )),
            );
        }
    }

    fn render(&self) {
        let renderer = &self.renderer.borrow();
        renderer.clear();

        // Render background
        self.starfield.borrow().render(renderer);

        // Round number
        self.round_text
            .render_hcenter(renderer.width() as f32, 10.0);

        // Player weapon selection boxes
        let player_box_w = renderer.width() as f32 * (3.0 / 4.0);
        let player_box_h = f32::min(
            (renderer.height() as f32 - self.round_text.height() - 10.0)
                / self.players.len() as f32
                - 20.0,
            renderer.height() as f32 / 4.0,
        );

        let player_box_x = (renderer.width() as f32 - player_box_w) / 2.0;
        let mut player_box_y = self.round_text.height()
            + ((renderer.height() as f32 - self.round_text.height())
                - ((player_box_h + 20.0) * self.players.len() as f32))
                / 2.0;

        for player in &self.players {
            self.render_player_box(
                player,
                RectF::new(player_box_x, player_box_y, player_box_w, player_box_h),
            );
            player_box_y += player_box_h + 20.0;
        }

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

        match button {
            MenuButton::Back => {
                return StackableStateResult::Pop;
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
                }
            }
            MenuButton::Right(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr)
                    && !p.decided
                {
                    p.selection = (p.selection + 1) % weapon_count;
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

    fn receive_return(&mut self, _retval: Box<dyn std::any::Any>) -> Result<()> {
        Err(anyhow!("Weapon selection state did not expect a return"))
    }

    fn resize_screen(&mut self) {
        self.starfield
            .borrow_mut()
            .update_screensize(self.renderer.borrow().size());

        let ww = Self::flavortext_max_width(self.renderer.borrow().width());
        self.weapon_texts
            .iter_mut()
            .for_each(|(_, t)| t.set_wrapwidth(ww));
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        // Animate background
        self.starfield.borrow_mut().step(timestep);

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
                            .map(|p| self.assets.weapons[p.selection].name.clone())
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
