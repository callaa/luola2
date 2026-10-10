use anyhow::{Result, anyhow};

use crate::{
    game::level::{LevelInfo, TILE_SIZE, procedural::generate_procedural_level},
    gfx::Image,
};

use std::sync::mpsc;
use std::thread;

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

pub struct PreloadingLevel {
    level: Option<Result<PreloadedLevel>>,
    recv: mpsc::Receiver<Result<PreloadedLevel>>,
}

pub enum PreloadingResult<'a> {
    Ok(&'a PreloadedLevel),
    Failed,
    Awaiting,
}

impl PreloadingLevel {
    pub fn start(info: LevelInfo) -> Self {
        let (send, recv) = mpsc::channel();

        thread::spawn(move || {
            send.send(preload_level(info)).unwrap();
        });

        PreloadingLevel { level: None, recv }
    }

    pub fn try_level(&mut self) -> PreloadingResult<'_> {
        if self.level.is_some() {
            match self.level.as_ref().unwrap() {
                Ok(l) => PreloadingResult::Ok(l),
                Err(_) => PreloadingResult::Failed,
            }
        } else {
            match self.recv.try_recv() {
                Ok(l) => {
                    self.level = Some(l);
                    match self.level.as_ref().unwrap() {
                        Ok(l) => PreloadingResult::Ok(l),
                        Err(_) => PreloadingResult::Failed,
                    }
                }
                Err(mpsc::TryRecvError::Empty) => PreloadingResult::Awaiting,
                Err(mpsc::TryRecvError::Disconnected) => panic!("Level generation thread died!"),
            }
        }
    }

    pub fn take_level(self) -> Result<PreloadedLevel> {
        if let Some(level) = self.level {
            level
        } else {
            self.recv
                .recv()
                .expect("Level generation thread should have returned a result!")
        }
    }
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
