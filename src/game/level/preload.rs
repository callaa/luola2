use anyhow::{Result, anyhow};

use crate::{
    game::level::{LevelInfo, TILE_SIZE, procedural::generate_procedural_level},
    gfx::Image,
};

/**
 * Preloaded level artwork and terrain.
 *
 * A level can be preloaded and shown in the background of the
 * weapon selection screen.
 */
#[derive(Clone)]
pub struct PreloadedLevel {
    pub artwork: Image,
    pub terrain: Image,
    pub script_settings: toml::Table,
    pub info: LevelInfo,
}

pub fn preload_level(info: LevelInfo) -> Result<PreloadedLevel> {
    if info.is_generated() {
        generate_procedural_level(info)
    } else {
        let terrain = Image::from_file(info.terrain_path())?.ensure_index8()?;
        if terrain.width() % TILE_SIZE > 0 || terrain.height() % TILE_SIZE > 0 {
            return Err(anyhow!(
                "Level dimensions not a multiple of 64 ({}x{})",
                terrain.width(),
                terrain.height()
            ));
        }

        let artwork = Image::from_file(info.artwork_path())?.ensure_argb888()?;
        if artwork.width() != terrain.width() || artwork.height() != terrain.height() {
            return Err(anyhow!(
                "Level artwork size does not match terrain map size"
            ));
        }

        Ok(PreloadedLevel {
            artwork,
            terrain,
            script_settings: info.script_settings().clone(),
            info,
        })
    }
}
