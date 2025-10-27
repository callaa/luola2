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
use std::{cell::RefCell, fs::read_to_string, rc::Rc};

use crate::{
    fs::find_datafile_path,
    game::{
        GameControllerSet, GameInitConfig, MenuButton, level::LevelInfo,
        scripting::ScriptEnvironment,
    },
    gfx::Renderer,
    menu::AnimatedStarfield,
    states::{MainMenu, game_assets::GameAssets, game_state::GameState},
};

use super::{StackableState, StackableStateResult};

pub struct GameInitState {
    is_init: bool,
    assets: Rc<GameAssets>,
    launch_file: Option<String>,
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,
}

impl GameInitState {
    pub fn new(
        launch_file: Option<String>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Self {
        Self {
            is_init: false,
            assets: Rc::new(GameAssets::new()),
            launch_file,
            controllers,
            renderer,
        }
    }
}

fn load_resources(renderer: Rc<RefCell<Renderer>>) -> Result<Rc<GameAssets>> {
    renderer
        .borrow_mut()
        .load_fontset(&find_datafile_path(&["fonts/fonts.toml"])?)?;

    renderer
        .borrow_mut()
        .load_textures(&find_datafile_path(&["textures/textures.toml"])?)?;

    // Load list of levels
    let mut levels = LevelInfo::load_level_packs()?;

    if levels.is_empty() {
        return Err(anyhow!("No levels found!"));
    }

    levels.sort_by(|a, b| a.title().cmp(b.title()));

    // Load scripts and extract weapon list
    // The full API isn't initialized and shouldn't be needed
    // just to load the scripts without executing the entrypoint function
    let lua = ScriptEnvironment::create_lua(renderer.clone())?;

    lua.load(r#"require "luola_main""#).exec()?;

    let secondary_weapon_table = lua
        .globals()
        .get::<mlua::Table>("luola_secondary_weapons")?;

    let mut weapons: Vec<_> = secondary_weapon_table
        .pairs::<String, mlua::Table>()
        //.map(|pair| pair.unwrap())
        .map(|pair| {
            let (k, v) = pair?;
            let title = v.get::<String>("title")?;

            Ok((k, title))
        })
        .collect::<Result<Vec<_>>>()?;

    if weapons.is_empty() {
        return Err(anyhow!("No weapons defined!"));
    }

    weapons.sort_by(|a, b| a.1.cmp(&b.1));

    Ok(Rc::new(GameAssets { levels, weapons }))
}

impl StackableState for GameInitState {
    fn receive_return(&mut self, _retval: Box<dyn std::any::Any>) -> Result<()> {
        Err(anyhow!("Init state did not expect a return with value!"))
    }

    fn handle_menu_button(&mut self, _button: MenuButton) -> StackableStateResult {
        StackableStateResult::Continue
    }

    fn resize_screen(&mut self) {}

    fn state_iterate(&mut self, _timestep: f32) -> StackableStateResult {
        if self.is_init {
            return StackableStateResult::Pop;
        }

        self.is_init = true;
        self.assets = match load_resources(self.renderer.clone()) {
            Ok(a) => a,
            Err(err) => return StackableStateResult::Error(err),
        };

        if let Some(launch) = &self.launch_file {
            // Direct launch game
            let conffile = match read_to_string(launch) {
                Ok(cf) => cf,
                Err(err) => {
                    return StackableStateResult::Error(err.into());
                }
            };

            let config: GameInitConfig = match toml::from_str(&conffile) {
                Ok(t) => t,
                Err(err) => {
                    return StackableStateResult::Error(err.into());
                }
            };

            let screen_size = self.renderer.borrow().size();

            match GameState::new_from_config(
                config,
                self.assets.clone(),
                Rc::new(RefCell::new(AnimatedStarfield::new(
                    200,
                    screen_size.0 as f32,
                    screen_size.1 as f32,
                ))),
                self.controllers.clone(),
                self.renderer.clone(),
            ) {
                Ok(g) => StackableStateResult::Replace(Box::new(g)),
                Err(err) => StackableStateResult::Error(err),
            }
        } else {
            // Open main menu
            let mainmenu = match MainMenu::new(
                self.assets.clone(),
                self.controllers.clone(),
                self.renderer.clone(),
            ) {
                Ok(mm) => mm,
                Err(e) => {
                    return StackableStateResult::Error(e);
                }
            };

            StackableStateResult::Replace(Box::new(mainmenu))
        }
    }
}
