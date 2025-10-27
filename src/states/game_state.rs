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
    gfx::Renderer,
    menu::AnimatedStarfield,
    states::{
        StackableState, StackableStateResult,
        game_assets::GameAssets,
        gameresults_state::GameResultsState,
        levelsel_state::LevelSelection,
        round_state::{GameRoundState, RoundWinner},
        weaponsel_state::{SelectedWeapons, WeaponSelection},
    },
};

/// The super-state for managing the different sub-states of the actual game
pub struct GameState {
    assets: Rc<GameAssets>,
    starfield: Rc<RefCell<AnimatedStarfield>>,
    players: Vec<Player>,
    level: Option<LevelInfo>,
    rounds_to_win: i32,
    round_winners: Vec<PlayerId>,
    substate: GameSubState,
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,
}

#[derive(PartialEq)]
enum GameSubState {
    SelectLevel,
    SelectWeapons,
    PlayRound,
    GameResults,
}

impl GameState {
    pub fn new(
        assets: Rc<GameAssets>,
        players: Vec<Player>,
        rounds_to_win: i32,
        starfield: Rc<RefCell<AnimatedStarfield>>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Self {
        Self {
            assets,
            starfield,
            players,
            rounds_to_win,
            round_winners: Vec::new(),
            level: None,
            substate: GameSubState::SelectLevel,
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
        let mut substate = GameSubState::SelectLevel;

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
                .map(|l| l.clone())
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

        let rounds_to_win = config.rounds.unwrap_or(1);

        Ok(Self {
            assets,
            starfield,
            players,
            level,
            rounds_to_win,
            round_winners,
            substate,
            controllers,
            renderer,
        })
    }
}

impl StackableState for GameState {
    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> Result<()> {
        if let Some(level) = retval.downcast_ref::<LevelInfo>() {
            self.level = Some(level.clone());
            self.substate = GameSubState::SelectWeapons;
        } else if let Some(weapons) = retval.downcast_ref::<SelectedWeapons>() {
            self.players.iter_mut().zip(&weapons.0).for_each(|(p, w)| {
                p.weapon = w.clone();
            });
            self.substate = GameSubState::PlayRound;
        } else if let Some(winner) = retval.downcast_ref::<RoundWinner>() {
            self.round_winners.push(winner.0);
            if winner.0 > 0 {
                let plr = &mut self.players[winner.0 as usize - 1];
                plr.wins += 1;
            }

            if self.players.iter().any(|p| p.wins >= self.rounds_to_win) {
                self.substate = GameSubState::GameResults;
            } else {
                self.substate = GameSubState::SelectLevel;
            }
        } else {
            return Err(anyhow!(
                "Unhandled game state return type: {:?}",
                retval.type_id()
            ));
        }

        Ok(())
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
            GameSubState::SelectLevel => {
                self.substate = GameSubState::GameResults;
                StackableStateResult::Push(Box::new(
                    match LevelSelection::new(
                        self.assets.clone(),
                        self.round_winners.len() as i32 + 1,
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
            GameSubState::SelectWeapons => {
                self.substate = GameSubState::SelectLevel;
                StackableStateResult::Push(Box::new(
                    match WeaponSelection::new(
                        self.assets.clone(),
                        &self.players,
                        self.round_winners.len() as i32 + 1,
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
            GameSubState::PlayRound => {
                self.substate = GameSubState::GameResults;

                return StackableStateResult::Push(Box::new(
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
                ));
            }
            GameSubState::GameResults => {
                return StackableStateResult::Replace(Box::new(
                    match GameResultsState::new(
                        take(&mut self.players),
                        take(&mut self.round_winners),
                        self.renderer.clone(),
                    ) {
                        Ok(r) => r,
                        Err(err) => return StackableStateResult::Error(err),
                    },
                ));
            }
        }
    }
}
