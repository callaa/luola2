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
    game::{MenuButton, Player, PlayerId},
    gfx::{Color, Renderer, Text, Texture, make_button_icon},
    math::{RectF, Vec2},
    menu::AnimatedStarfield,
    states::game_assets::GameAssets,
};

pub struct WeaponSelection {
    starfield: Rc<RefCell<AnimatedStarfield>>,
    assets: Rc<GameAssets>,
    round_text: Text,
    renderer: Rc<RefCell<Renderer>>,
    players: Vec<PlayerWeaponChoice>,
    weapon_texts: Vec<Text>,
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
        weapon_list: &[(String, String)],
        renderer: &Renderer,
    ) -> Result<Self> {
        let selection = weapon_list
            .iter()
            .enumerate()
            .find(|(_, (n, _))| n == &player.weapon)
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        Ok(Self {
            controller: player.controller as usize - 1,
            selection,
            player_text: renderer
                .fontset()
                .menu
                .create_text(renderer, &format!("Player {}:", player_id))?
                .with_color(Color::player_color(player_id)),
            left_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Left,
                renderer,
            )?,
            right_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Right,
                renderer,
            )?,
            select_button_icon: make_button_icon(
                player.controller,
                crate::game::MappedKey::Fire1,
                renderer,
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
    ) -> Result<Self> {
        let round_text = renderer
            .borrow()
            .fontset()
            .menu_big
            .create_text(&renderer.borrow(), &format!("Round {}", round))?
            .with_color(Color::new(0.9, 0.2, 0.2));

        let weapon_texts = assets
            .weapons
            .iter()
            .map(|(_, title)| {
                renderer
                    .borrow()
                    .fontset()
                    .menu
                    .create_text(&renderer.borrow(), title)
            })
            .collect::<Result<Vec<_>>>()?;

        let longest_weapon_text_width = weapon_texts
            .iter()
            .fold(0.0, |acc, t| f32::max(acc, t.width()));

        let choices: Result<Vec<_>> = players
            .iter()
            .enumerate()
            .map(|(idx, player)| {
                PlayerWeaponChoice::from_weapon_name(
                    player,
                    idx as PlayerId + 1,
                    &assets.weapons,
                    &renderer.borrow(),
                )
            })
            .collect();

        Ok(Self {
            assets,
            weapon_texts,
            players: choices?,
            round_text,
            starfield,
            renderer,
            longest_weapon_text_width,
            start_timer: None,
        })
    }

    fn find_player_mut(&mut self, controller: i32) -> Option<&mut PlayerWeaponChoice> {
        assert!(controller > 0);
        let controller = (controller - 1) as usize;
        self.players.iter_mut().find(|p| p.controller == controller)
    }

    fn render(&self) {
        let renderer = &self.renderer.borrow();
        renderer.clear();

        // Render background
        self.starfield.borrow().render(renderer);

        // Round number
        self.round_text
            .render_hcenter(renderer.width() as f32, 10.0);

        // Player weapon selections
        let line_height = self.players[0]
            .left_button_icon
            .height()
            .max(self.players[0].player_text.height());

        let x = (renderer.width() as f32
            - (self.players[0].player_text.width()
                + self.players[0].left_button_icon.width() * 3.0
                + self.longest_weapon_text_width
                + 30.0))
            / 2.0;
        let mut y = (renderer.height() as f32 - (line_height * self.players.len() as f32)) / 2.0;

        for plr in &self.players {
            let mut x = x;
            let center = (line_height - plr.player_text.height()) / 2.0;
            plr.player_text.render(Vec2(x, y + center));
            x += plr.player_text.width() + 10.0;

            if !plr.decided {
                plr.left_button_icon.render_simple(
                    renderer,
                    None,
                    Some(RectF::new(
                        x,
                        y,
                        plr.left_button_icon.width(),
                        plr.left_button_icon.height(),
                    )),
                );
                x += plr.left_button_icon.width() + 10.0;
            }

            self.weapon_texts[plr.selection].render(Vec2(x, y + center));
            x += self.weapon_texts[plr.selection].width() + 10.0;

            if !plr.decided {
                plr.right_button_icon.render_simple(
                    renderer,
                    None,
                    Some(RectF::new(
                        x,
                        y,
                        plr.right_button_icon.width(),
                        plr.right_button_icon.height(),
                    )),
                );
                x += plr.right_button_icon.width() + 10.0;
                plr.select_button_icon.render_simple(
                    renderer,
                    None,
                    Some(RectF::new(
                        x,
                        y,
                        plr.select_button_icon.width(),
                        plr.select_button_icon.height(),
                    )),
                );
            }

            y += line_height;
        }

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
                if let Some(p) = self.find_player_mut(plr) {
                    if !p.decided {
                        if p.selection > 0 {
                            p.selection -= 1;
                        } else {
                            p.selection = weapon_count - 1;
                        }
                    }
                }
            }
            MenuButton::Right(plr) if plr > 0 => {
                if let Some(p) = self.find_player_mut(plr) {
                    if !p.decided {
                        p.selection = (p.selection + 1) % weapon_count;
                    }
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
                            .map(|p| self.assets.weapons[p.selection].0.clone())
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
