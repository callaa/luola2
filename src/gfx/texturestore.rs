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

use crate::gfx::{Renderer, TextureConfigWithAlts};

use super::Texture;
use anyhow::{Result, anyhow};
use mlua;
use sdl3_sys::render::SDL_Texture;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

/**
 * Storage for shared textures that are kept loaded for the duration of the application run.
 */
pub struct TextureStore {
    textures: Vec<TextureWithAlts>,
    name_map: HashMap<String, TextureId>,
}

struct TextureWithAlts {
    main: Texture,
    // alt textures have all the same settings as the main texture,
    // with the possible exception of the texture reference and subrect.
    decal: Option<Texture>,
    active: Option<Texture>,
    damage: Option<Texture>,
}

const TEXTURE_FLAG_NEED_ROTATION: u16 = 0x1000; // sprite should be rotated to look natural
const TEXTURE_FLAG_FLIPPABLE: u16 = 0x2000; // sprite should be flipped when moving to the left
const TEXTURE_FLAG_FRAME_MASK: u16 = 0x0fff; // part of the flags that represents frame count

#[derive(Clone, Copy, Debug)]
pub struct TextureId {
    offset: u16,         // offset to texture array
    flags: u16,          // extra info that may be needed
    frame_duration: f32, // length of animation (0 if not animated)
}

impl TextureId {
    fn from(offset: usize, tex: &Texture) -> Self {
        debug_assert!(offset <= 0xffff);
        debug_assert!(tex.frames <= TEXTURE_FLAG_FRAME_MASK as i32);
        Self {
            offset: offset as u16,
            flags: (tex.frames as u16) & TEXTURE_FLAG_FRAME_MASK
                | if tex.needs_rotation() {
                    TEXTURE_FLAG_NEED_ROTATION
                } else {
                    0
                }
                | if tex.flippable() {
                    TEXTURE_FLAG_FLIPPABLE
                } else {
                    0
                },
            frame_duration: tex.frame_duration,
        }
    }

    pub fn frames(&self) -> i32 {
        (self.flags & TEXTURE_FLAG_FRAME_MASK) as i32
    }

    pub fn frame_duration(&self) -> f32 {
        self.frame_duration
    }

    pub fn needs_rotation(&self) -> bool {
        (self.flags & TEXTURE_FLAG_NEED_ROTATION) != 0
    }

    pub fn flippable(&self) -> bool {
        (self.flags & TEXTURE_FLAG_FLIPPABLE) != 0
    }
}
impl mlua::UserData for TextureId {}

impl mlua::FromLua for TextureId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "TextureId".to_owned(),
                message: Some("expected TextureId".to_string()),
            }),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Hash, Clone, Copy)]
pub enum TexAlt {
    Main,   // the main texture
    Decal,  // intended to be tinted and drawn over the main texture
    Active, // object in active state (thrusting, flying, walking, etc.)
    Damage, // in the process of taking damage
}

impl Eq for TexAlt {}

impl TextureStore {
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            name_map: HashMap::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.textures.len()
    }

    pub fn load_from_toml(renderer: &Renderer, path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: HashMap<String, TextureConfigWithAlts> = toml::from_str(&content)?;

        let mut store = Self::new();

        let root = path
            .parent()
            .expect("textures.toml should have a parent directory");

        let mut shared_textures: HashMap<String, *mut SDL_Texture> = HashMap::new();

        for (name, config) in config {
            let main = store
                .add_texture(
                    name,
                    Texture::from_config(renderer, root, &config.main, None, &mut shared_textures)?,
                )
                .expect("duplicates shouldn't be possible here");

            if let Some(alts) = config.alts {
                for (alt, altconfig) in alts {
                    store.add_texture_alt(
                        main,
                        alt,
                        Texture::from_config(
                            renderer,
                            root,
                            &config.main,
                            Some(&altconfig),
                            &mut shared_textures,
                        )?,
                    )?;
                }
            }
        }
        Ok(store)
    }

    pub fn add_texture(&mut self, name: String, texture: Texture) -> Result<TextureId> {
        if self.name_map.contains_key(&name) {
            return Err(anyhow!("Texture {} already added", name));
        }

        self.textures.push(TextureWithAlts {
            main: texture,
            decal: None,
            active: None,
            damage: None,
        });
        let id = TextureId::from(self.textures.len() - 1, &self.textures.last().unwrap().main);
        self.name_map.insert(name, id);
        Ok(id)
    }

    pub fn add_texture_alt(
        &mut self,
        main: TextureId,
        alt: TexAlt,
        texture: Texture,
    ) -> Result<()> {
        let tex = &mut self.textures[main.offset as usize];
        let at = match alt {
            TexAlt::Main => return Err(anyhow!("Main texture already added!")),
            TexAlt::Decal => &mut tex.decal,
            TexAlt::Active => &mut tex.active,
            TexAlt::Damage => &mut tex.damage,
        };
        if let Some(_) = at {
            return Err(anyhow!("Texture alt {:?} already set!", alt));
        }

        *at = Some(texture);

        Ok(())
    }

    pub fn find_texture(&self, name: &str) -> Result<TextureId> {
        self.name_map
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("Texture \"{}\" not found", name))
    }

    pub fn get_texture(&self, id: TextureId) -> &Texture {
        &self.textures[id.offset as usize].main
    }

    pub fn get_texture_alt(&self, id: TextureId, alt: TexAlt) -> Option<&Texture> {
        let tex = &self.textures[id.offset as usize];
        match alt {
            TexAlt::Main => Some(&tex.main),
            TexAlt::Decal => tex.decal.as_ref(),
            TexAlt::Active => tex.active.as_ref(),
            TexAlt::Damage => tex.damage.as_ref(),
        }
    }

    /// Get the named alt texture, or the main texture if alt not set
    pub fn get_texture_alt_fallback(&self, id: TextureId, alt: TexAlt) -> &Texture {
        let tex = &self.textures[id.offset as usize];
        match alt {
            TexAlt::Main => Some(&tex.main),
            TexAlt::Decal => tex.decal.as_ref(),
            TexAlt::Active => tex.active.as_ref(),
            TexAlt::Damage => tex.damage.as_ref(),
        }
        .unwrap_or(&tex.main)
    }
}
