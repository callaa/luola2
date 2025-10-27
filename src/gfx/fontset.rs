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

use std::fs;
use std::path::Path;

use super::Font;
use anyhow::Result;
use serde::Deserialize;

pub struct FontSet {
    pub menu: Font,
    pub menu_big: Font,
}

#[derive(Deserialize)]
struct FontOptions {
    file: String,
    size: f32,
}

#[derive(Deserialize)]
struct FontSetConfig {
    menu: FontOptions,
    menu_big: FontOptions,
}

impl FontSet {
    pub fn load_from_toml(config_file: &Path) -> Result<FontSet> {
        let content = fs::read_to_string(config_file)?;
        let config: FontSetConfig = toml::from_str(&content)?;

        let root = config_file
            .parent()
            .expect("fonts.toml should have a parent directory");

        Ok(Self {
            menu: config.menu.load(root)?,
            menu_big: config.menu_big.load(root)?,
        })
    }
}

impl FontOptions {
    fn load(&self, root: &Path) -> Result<Font> {
        Font::from_file([root, Path::new(&self.file)].iter().collect(), self.size)
    }
}
