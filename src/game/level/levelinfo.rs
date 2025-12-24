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

use log::error;
use sdl3_sys::pixels::{SDL_GetPixelFormatDetails, SDL_MapRGBA, SDL_Palette, SDL_PixelFormat};
use serde;
use std::{
    fs,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    ptr::null,
};
use toml;

use anyhow::{Result, anyhow};

use super::terrain::*;
use crate::{
    fs::glob_datafiles,
    game::level::LEVEL_SCALE,
    gfx::{Renderer, Texture},
    math::RectF,
};

#[derive(Clone)]
pub struct LevelInfo {
    root: PathBuf,
    levelpack: String,
    name: String,
    title: String,
    artwork_file: String,
    terrain_file: String,
    thumbnail: Option<Texture>,
    background_file: Option<String>,
    script_file: Option<String>,
    terrain_palette: TerrainPalette,
    colors: TerrainColors,
    transparent_color_index: Option<u8>,
    script_settings: toml::Table,
    starfield: bool,
    nospawnzones: Vec<RectF>,
}

type TerrainPalette = [u8; 256];

#[derive(serde::Deserialize, Debug)]
struct LevelInfoToml {
    title: String,
    terrain: String,
    artwork: String,
    thumbnail: String,
    background: Option<String>,
    script: Option<String>,

    #[serde(default)]
    starfield: bool,

    #[serde(default)]
    nospawnzones: Vec<NoSpawnZoneToml>,

    #[serde(rename = "terrain-palette")]
    terrain_palette: toml::Table,

    #[serde(default)]
    colors: TerrainColors,

    #[serde(rename = "script-settings")]
    script_settings: Option<toml::Table>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct NoSpawnZoneToml {
    rect: (i32, i32, i32, i32),
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
struct TerrainColors {
    water: Option<u32>,
    snow: Option<u32>,
}

impl LevelInfo {
    pub fn load(path: &Path, renderer: &Renderer) -> Result<LevelInfo> {
        let content = fs::read_to_string(path)?;
        let info: LevelInfoToml = toml::from_str(&content)?;
        let root = path
            .parent()
            .expect("level info file path has no parent?")
            .to_owned();

        let terrain_palette = parse_palette_mapping(&info.terrain_palette)?;
        // Find the first color mapped to free space. This will be used
        // as transparency key if the terrain artwork is the same as the terrain map
        let transparent_color_index = terrain_palette
            .iter()
            .enumerate()
            .find(|(_, p)| **p == 0)
            .map(|(idx, _)| idx as u8);

        let thumbnail = match Texture::from_file(renderer, root.join(info.thumbnail)) {
            Ok(t) => Some(t),
            Err(err) => {
                log::warn!("Couldn't load thumbnail: {}", err);
                None
            }
        };

        let nospawnzones = info
            .nospawnzones
            .iter()
            .map(|n| {
                RectF::new(
                    n.rect.0 as f32 * LEVEL_SCALE,
                    n.rect.1 as f32 * LEVEL_SCALE,
                    n.rect.2 as f32 * LEVEL_SCALE,
                    n.rect.3 as f32 * LEVEL_SCALE,
                )
            })
            .collect();

        Ok(LevelInfo {
            root,
            levelpack: path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
            name: path.file_stem().unwrap().to_str().unwrap().to_owned(),
            title: info.title,
            artwork_file: info.artwork,
            terrain_file: info.terrain,
            thumbnail,
            background_file: info.background,
            script_file: info.script,
            terrain_palette,
            transparent_color_index,
            script_settings: info.script_settings.unwrap_or_default(),
            starfield: info.starfield,
            colors: info.colors,
            nospawnzones,
        })
    }

    pub fn load_level_packs(renderer: &Renderer) -> Result<Vec<LevelInfo>> {
        let files = glob_datafiles("levels", "*/*.toml")?;

        Ok(files
            .iter()
            .filter_map(|f| match LevelInfo::load(f, renderer) {
                Ok(l) => Some(l),
                Err(err) => {
                    error!("Couldn't load level info: {}", err);
                    None
                }
            })
            .collect())
    }

    pub fn levelpack(&self) -> &str {
        &self.levelpack
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn script_settings(&self) -> &toml::Table {
        &self.script_settings
    }

    pub fn terrain_path(&self) -> PathBuf {
        self.root.join(&self.terrain_file)
    }

    pub fn artwork_path(&self) -> PathBuf {
        self.root.join(&self.artwork_file)
    }

    pub fn terrain_is_same_as_artwork(&self) -> bool {
        self.terrain_file == self.artwork_file
    }

    pub fn thumbnail(&self) -> Option<&Texture> {
        self.thumbnail.as_ref()
    }

    pub fn background_path(&self) -> Option<PathBuf> {
        self.background_file.as_ref().map(|f| self.root.join(f))
    }

    pub fn script_path(&self) -> Option<PathBuf> {
        self.script_file.as_ref().map(|f| self.root.join(f))
    }

    pub fn use_starfield(&self) -> bool {
        self.starfield
    }

    pub fn nospawnzones(&self) -> &Vec<RectF> {
        &self.nospawnzones
    }

    // Convert the given pixel values into the internal format using the terrain palette map
    pub fn map_palette(&self, pixels: &mut [u8]) {
        for p in pixels {
            *p = self.terrain_palette[*p as usize];
        }
    }

    pub fn transparent_color_index(&self) -> Option<u8> {
        self.transparent_color_index
    }

    pub(super) fn find_water_color(
        &self,
        palette: &SDL_Palette,
        pixelformat: SDL_PixelFormat,
    ) -> Option<u32> {
        if self.colors.water.is_some() {
            return self.colors.water;
        }

        for (idx, terrain) in self
            .terrain_palette
            .iter()
            .enumerate()
            .take(palette.ncolors as usize)
        {
            if is_water(*terrain) {
                let pfd = unsafe { SDL_GetPixelFormatDetails(pixelformat) };
                let color = unsafe { palette.colors.add(idx).as_ref().unwrap() };
                return Some(unsafe {
                    SDL_MapRGBA(pfd, null(), color.r, color.g, color.b, color.a)
                });
            }
        }
        None
    }

    pub(super) fn get_snow_color(&self) -> u32 {
        self.colors.snow.unwrap_or(0xffffffff)
    }
}

fn parse_palette_mapping(table: &toml::Table) -> Result<TerrainPalette> {
    // Default is to map all unmapped colors to solid ground.
    // This means we should have at least one free-space mapping
    // for the level to be playable
    let mut mapping: TerrainPalette = [TER_BIT_DESTRUCTIBLE | TER_TYPE_GROUND; 256];

    for (key, value) in table.iter() {
        let mut mods_set: u8 = 0;
        let mut mods_clear: u8 = 0;

        let mut parts = key.split('-');
        let name = parts.next().unwrap();
        for part in parts {
            if part == "uw" {
                mods_set |= TER_BIT_WATER;
            } else if part == "i" {
                mods_clear |= TER_BIT_DESTRUCTIBLE;
            } else {
                return Err(anyhow!("Unknown terrain type modifier: {}", part));
            }
        }

        let terrain_type = mods_set
            | match name {
                "space" => 0,
                "water" => TER_BIT_WATER,        // shorthand for "space-uw"
                "paint" => TER_BIT_DESTRUCTIBLE, // free-space pixel with erasable artwork
                "ground" => TER_TYPE_GROUND | TER_BIT_DESTRUCTIBLE,
                "burnable" => TER_TYPE_BURNABLE | TER_BIT_DESTRUCTIBLE,
                "cinder" => TER_TYPE_CINDER | TER_BIT_DESTRUCTIBLE,
                "explosive" => TER_TYPE_EXPLOSIVE | TER_BIT_DESTRUCTIBLE,
                "highexplosive" => TER_TYPE_HIGH_EXPLOSIVE | TER_BIT_DESTRUCTIBLE,
                "ice" => TER_TYPE_ICE | TER_BIT_DESTRUCTIBLE,
                "base" => TER_TYPE_BASE | TER_BIT_DESTRUCTIBLE,
                "basesupport" => TER_TYPE_BASESUPPORT | TER_BIT_DESTRUCTIBLE,
                "noregenbase" => TER_TYPE_NOREGENBASE | TER_BIT_DESTRUCTIBLE,
                "walkway" => TER_TYPE_WALKWAY | TER_BIT_DESTRUCTIBLE,
                "greygoo" => TER_TYPE_GREYGOO | TER_BIT_DESTRUCTIBLE,
                "damage" => TER_TYPE_DAMAGE | TER_BIT_DESTRUCTIBLE,
                _ => {
                    return Err(anyhow!("Unknown terrain type: {}", name));
                }
            } & !mods_clear;

        match value {
            toml::Value::String(v) => {
                mapping[parse_range(v)?].fill(terrain_type);
            }
            toml::Value::Integer(v) => {
                if *v < 0 || *v > 255 {
                    return Err(anyhow!("Index out of range (0-255)"));
                }

                mapping[*v as usize] = terrain_type;
            }
            toml::Value::Array(a) => {
                for av in a {
                    match av {
                        toml::Value::String(v) => {
                            mapping[parse_range(v)?].fill(terrain_type);
                        }
                        toml::Value::Integer(av) => {
                            if *av < 0 || *av > 255 {
                                return Err(anyhow!("Index out of range (0-255)"));
                            }

                            mapping[*av as usize] = terrain_type;
                        }
                        _ => {
                            return Err(anyhow!("Expected number or range"));
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow!(
                    "Index mapping should be either a number, a range, or a list of numbers and ranges"
                ));
            }
        }
    }

    Ok(mapping)
}

fn parse_range(rangestr: &str) -> Result<RangeInclusive<usize>> {
    let sep = rangestr.find('-').ok_or(anyhow!("invalid range"))?;

    let start = rangestr[0..sep].parse::<usize>()?;
    let end = rangestr[sep + 1..].parse::<usize>()?;

    if start > 255 || end > 255 {
        return Err(anyhow!("Palette indexes must be in range 0-255"));
    }

    Ok(start..=end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_parsing() {
        let palette_section: toml::Table = toml::from_str(
            r#"
            space = 0
            water = 1
            ground-uw = 2
            base-uw-i = 3
            burnable = [4, 5]
            cinder = "6-8"
            explosive = ["9-10"]
        "#,
        )
        .unwrap();

        let mapping = parse_palette_mapping(&palette_section).unwrap();

        assert_eq!(mapping[0], 0);
        assert_eq!(mapping[1], TER_BIT_WATER);
        assert_eq!(
            mapping[2],
            TER_BIT_WATER | TER_BIT_DESTRUCTIBLE | TER_TYPE_GROUND
        );
        assert_eq!(mapping[3], TER_BIT_WATER | TER_TYPE_BASE);
        assert_eq!(mapping[4], TER_BIT_DESTRUCTIBLE | TER_TYPE_BURNABLE);
        assert_eq!(mapping[5], TER_BIT_DESTRUCTIBLE | TER_TYPE_BURNABLE);
        assert_eq!(mapping[6], TER_BIT_DESTRUCTIBLE | TER_TYPE_CINDER);
        assert_eq!(mapping[7], TER_BIT_DESTRUCTIBLE | TER_TYPE_CINDER);
        assert_eq!(mapping[8], TER_BIT_DESTRUCTIBLE | TER_TYPE_CINDER);
        assert_eq!(mapping[9], TER_BIT_DESTRUCTIBLE | TER_TYPE_EXPLOSIVE);
        assert_eq!(mapping[10], TER_BIT_DESTRUCTIBLE | TER_TYPE_EXPLOSIVE);

        for i in 11..256 {
            assert_eq!(mapping[i], TER_BIT_DESTRUCTIBLE | TER_TYPE_GROUND);
        }
    }
}
