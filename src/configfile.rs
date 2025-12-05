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

use std::{
    fs::{read_to_string, write},
    sync::RwLock,
};

use log::{error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{events::push_config_changed_event, fs::get_savefile_path, game::PlayerKeymap};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct VideoConfig {
    #[serde(default)]
    pub fullscreen: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameOptions {
    #[serde(default = "default_true")]
    pub minimap: bool,
    #[serde(default = "default_true")]
    pub baseregen: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            minimap: true,
            baseregen: true,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserConfig {
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub game: GameOptions,
    pub keymap1: Option<PlayerKeymap>,
    pub keymap2: Option<PlayerKeymap>,
    pub keymap3: Option<PlayerKeymap>,
    pub keymap4: Option<PlayerKeymap>,
}

pub static GAME_CONFIG: RwLock<UserConfig> = RwLock::new(UserConfig {
    video: VideoConfig { fullscreen: false },
    game: GameOptions {
        minimap: true,
        baseregen: true,
    },
    keymap1: None,
    keymap2: None,
    keymap3: None,
    keymap4: None,
});

pub fn load_user_config() {
    let filename = get_savefile_path("settings.toml");
    let content = match read_to_string(&filename) {
        Ok(c) => c,
        Err(e) => {
            warn!("Couldn't read user config file ({:?}): {}", filename, e);
            "".to_owned()
        }
    };

    let config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            error!("Couldn't parse user config file ({:?}: {}", filename, e);
            Default::default()
        }
    };

    let mut w = GAME_CONFIG.write().unwrap();
    *w = config;
}

pub fn save_user_config(config: UserConfig) {
    let filename = get_savefile_path("settings.toml");
    let content = match toml::to_string(&config) {
        Ok(c) => c,
        Err(err) => {
            error!("Failed to serialize user config! {err}");
            return;
        }
    };

    if let Err(e) = write(&filename, content) {
        error!("Failed to write config file {:?}: {e}", filename);
        return;
    }

    let mut w = GAME_CONFIG.write().unwrap();
    *w = config;
    drop(w);

    info!("Saved user preferences {:?}", filename);
    push_config_changed_event();
}
