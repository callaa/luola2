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

use anyhow::{Result, anyhow};
use std::{cell::RefCell, mem::take, rc::Rc};

use crate::{
    game::{GameControllerSet, GameInitConfig, MenuButton, Player, PlayerId, level::LevelInfo},
    gfx::{Renderer, Texture},
    menu::AnimatedStarfield,
    states::{
        StackableState, StackableStateResult,
        game_assets::GameAssets,
        gameresults_state::GameResultsState,
        levelsel_state::LevelSelection,
        round_state::{GameRoundState, RoundWinner},
        roundresults_state::RoundResultsState,
        weaponsel_state::{SelectedWeapons, WeaponSelection},
    },
};

/// The super-state for managing the different sub-states of the actual game
pub struct GameState {
    assets: Rc<GameAssets>,
    starfield: Rc<RefCell<AnimatedStarfield>>,
    players: Vec<Player>,
    level: Option<LevelInfo>,
    rounds: i32,
    round_winners: Vec<PlayerId>,
    substate: GameSubState,
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,
}

#[derive(PartialEq)]
enum GameSubState {
    SelectLevel,     // return from SelectWeapons state without fadein animation
    SelectNextLevel, // same as SelectLevel except use fadein animation
    SelectWeapons,
    PlayRound,
    RoundResults,
    GameResults,
}

impl GameState {
    pub fn new(
        assets: Rc<GameAssets>,
        players: Vec<Player>,
        rounds: i32,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Self {
        Self {
            assets,
            starfield,
            players,
            rounds,
            round_winners: Vec::new(),
            level: None,
            substate: GameSubState::SelectNextLevel,
            controllers,
            renderer,
        }
    }

    pub fn new_from_config(
        config: GameInitConfig,
        assets: Rc<GameAssets>,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let mut substate = GameSubState::SelectNextLevel;

        let level = if config.gameover.unwrap_or(false) {
            // Gameover: jump straight to the game over screen
            // (obviously, this is for testing/debugging purposes)
            substate = GameSubState::GameResults;

            None
        } else if config.level.is_empty() {
            None
        } else {
            // Level set, skip selector
            substate = GameSubState::SelectWeapons;

            assets
                .levels
                .iter()
                .find(|l| l.name() == config.level)
                .cloned()
        };

        let round_winners = config.winners;
        let mut players = config.players;
        for winner in &round_winners {
            if *winner > 0 {
                players[*winner as usize - 1].wins += 1;
            }
        }

        if substate == GameSubState::SelectWeapons && players.iter().all(|p| !p.weapon.is_empty()) {
            // level and weapons set, skip weapon selector too
            substate = GameSubState::PlayRound;
        }

        let rounds = config.rounds.unwrap_or(1);

        Ok(Self {
            assets,
            starfield,
            players,
            level,
            rounds,
            round_winners,
            substate,
            controllers,
            renderer,
        })
    }
}

impl StackableState for GameState {
    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        if let Some(level) = retval.downcast_ref::<LevelInfo>() {
            self.level = Some(level.clone());
            self.substate = GameSubState::SelectWeapons;
        } else if let Some(weapons) = retval.downcast_ref::<SelectedWeapons>() {
            self.players.iter_mut().zip(&weapons.0).for_each(|(p, w)| {
                p.ship = w.0.clone();
                p.weapon = w.1.clone();
            });
            self.substate = GameSubState::PlayRound;
        } else if let Some(winner) = retval.downcast_ref::<RoundWinner>() {
            if winner.0 > 0 {
                let plr = &mut self.players[winner.0 as usize - 1];
                plr.wins += 1;
            }
            self.round_winners.push(winner.0);
            if winner.1 || self.round_winners.len() as i32 >= self.rounds {
                self.substate = GameSubState::GameResults;
            } else {
                self.substate = GameSubState::RoundResults;
            }
        } else {
            return StackableStateResult::Error(anyhow!(
                "Unhandled game state return type: {:?}",
                retval.type_id()
            ));
        }

        StackableStateResult::Continue
    }

    fn handle_menu_button(&mut self, _button: MenuButton) -> StackableStateResult {
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {
        //self.game.relayout_viewports(&self.renderer.borrow());
    }

    fn state_iterate(&mut self, _timestep: f32) -> StackableStateResult {
        // This state is a manager state that creates the right sub-states
        // where the game is played.
        // By default, the next state is always GameResults to handle the case
        // where the player chooses to cancel the game early.
        match self.substate {
            GameSubState::RoundResults => {
                self.substate = GameSubState::SelectNextLevel;
                let last_winner = *self
                    .round_winners
                    .last()
                    .expect("last winner should have been set");
                StackableStateResult::Push(Box::new(
                    match RoundResultsState::new(
                        self.round_winners.len() as i32,
                        last_winner,
                        self.starfield.clone(),
                        self.renderer.clone(),
                    ) {
                        Ok(s) => s,
                        Err(err) => {
                            return StackableStateResult::Error(err);
                        }
                    },
                ))
            }
            GameSubState::SelectNextLevel | GameSubState::SelectLevel => {
                let fadein_round_text = matches!(self.substate, GameSubState::SelectNextLevel);
                self.substate = GameSubState::GameResults;
                let selection = if let Some(level) = &self.level {
                    self.assets.levels.iter().position(|l| {
                        l.levelpack() == level.levelpack() && l.name() == level.name()
                    })
                } else {
                    None
                }
                .unwrap_or(0);

                StackableStateResult::Push(Box::new(
                    match LevelSelection::new(
                        self.assets.clone(),
                        self.round_winners.len() as i32 + 1,
                        fadein_round_text,
                        self.starfield.clone(),
                        self.renderer.clone(),
                        selection,
                    ) {
                        Ok(s) => s,
                        Err(err) => {
                            return StackableStateResult::Error(err);
                        }
                    },
                ))
            }
            GameSubState::SelectWeapons => {
                self.substate = GameSubState::SelectLevel;
                let level_art = if let Some(lev) = &self.level {
                    match Texture::from_file(&self.renderer.borrow(), lev.artwork_path()) {
                        Ok(t) => Some(t),
                        Err(e) => {
                            log::error!("Couldn't open level art {:?}: {}", lev.artwork_path(), e);
                            None
                        }
                    }
                } else {
                    log::error!("Entered weapon selection state with no level set!");
                    None
                };

                StackableStateResult::Push(Box::new(
                    match WeaponSelection::new(
                        self.assets.clone(),
                        &self.players,
                        self.round_winners.len() as i32 + 1,
                        level_art,
                        self.starfield.clone(),
                        self.renderer.clone(),
                        &self.controllers.borrow(),
                    ) {
                        Ok(s) => s,
                        Err(err) => {
                            return StackableStateResult::Error(err);
                        }
                    },
                ))
            }
            GameSubState::PlayRound => {
                self.substate = GameSubState::GameResults;

                StackableStateResult::Push(Box::new(
                    match GameRoundState::new(
                        self.players.clone(),
                        self.level
                            .as_ref()
                            .expect("Level should have been loaded at this point"),
                        self.controllers.clone(),
                        self.renderer.clone(),
                    ) {
                        Ok(g) => g,
                        Err(err) => return StackableStateResult::Error(err),
                    },
                ))
            }
            GameSubState::GameResults => {
                if self.round_winners.is_empty() {
                    return StackableStateResult::Pop;
                }

                self.controllers.borrow().clear_player_leds();
                StackableStateResult::Replace(Box::new(
                    match GameResultsState::new(
                        take(&mut self.players),
                        take(&mut self.round_winners),
                        self.renderer.clone(),
                    ) {
                        Ok(r) => r,
                        Err(err) => return StackableStateResult::Error(err),
                    },
                ))
            }
        }
    }
}
