use super::mixer::{Audio, Mixer, PlayableSoundEffect};
use crate::{
    fs::{glob_datafiles, pathbuf_to_cstring},
    gfx::SdlError,
    math::Vec2,
};
use sdl3_mixer_sys::mixer::MIX_LoadAudio;
use std::{cell::RefCell, collections::HashMap, os::unix::ffi::OsStringExt, rc::Rc};

#[derive(Clone)]
pub struct SfxStore {
    mixer: Rc<RefCell<Mixer>>,
    sample_map: HashMap<Vec<u8>, Audio>,
    soundeffect_queue: Vec<PlayableSoundEffect>,
}

impl SfxStore {
    pub fn new(mixer: Rc<RefCell<Mixer>>) -> Self {
        Self {
            mixer,
            sample_map: HashMap::new(),
            soundeffect_queue: Vec::new(),
        }
    }

    pub fn get_mixer(&self) -> Rc<RefCell<Mixer>> {
        self.mixer.clone()
    }

    pub fn load_sound_effects(mut self) -> Self {
        let mixer = self.mixer.borrow().mixer;
        if mixer.is_null() {
            log::info!("SDL Mixer not initialized: not loading sound effects.");
            return self;
        }

        let files = match glob_datafiles("sounds", "*.ogg") {
            Ok(f) => f,
            Err(e) => {
                log::error!("Couldn't get sound files: {}", e);
                return self;
            }
        };

        for file in files {
            let name = file
                .as_path()
                .file_stem()
                .expect("valid filename")
                .to_owned();
            let pathstr = pathbuf_to_cstring(file).expect("valid path");
            let audio = unsafe { MIX_LoadAudio(mixer, pathstr.as_ptr(), true) };

            if audio.is_null() {
                SdlError::log("Couldn't load file");
            } else {
                log::debug!("Loaded sound effect: {:?}", name);
                self.sample_map.insert(name.into_vec(), Audio::new(audio));
            }
        }

        if self.sample_map.is_empty() {
            log::warn!("No sound effects found!");
        }
        self
    }

    /**
     * Find a sound effect
     *
     * Name is given as a byte slice for efficient use via Lua API.
     * Returns an invalid SoundEffectId if sample wasn't found.
     */
    pub fn find_soundeffect(&self, name: &[u8]) -> Audio {
        self.sample_map.get(name).cloned().unwrap_or_else(|| {
            if !self.sample_map.is_empty() {
                // don't bother spamming warnings if sounds are disabled
                log::warn!(
                    "Sound effect \"{}\" not found!",
                    str::from_utf8(name).unwrap()
                );
            }
            Audio::invalid()
        })
    }

    /// Immediately play an UI sound effect (not positional, uses blips soundtrack)
    pub fn play_blip(&self, name: &[u8]) {
        if let Some(sample) = self.sample_map.get(name) {
            self.mixer
                .borrow()
                .play_soundeffect(PlayableSoundEffect::Blip(sample.clone()));
        }
    }

    /// Immediately play a sound effect
    pub fn play_soundeffect(&self, sound: PlayableSoundEffect) {
        self.mixer.borrow().play_soundeffect(sound);
    }

    /// Add a sound effect to the playback queue
    pub fn enqueue_soundeffect(&mut self, sound: PlayableSoundEffect) {
        let mixer = self.mixer.borrow().mixer;
        if mixer.is_null() {
            return;
        }

        self.soundeffect_queue.push(sound);
    }

    /// Play as many queued sound effects as we have space for in the mixer.
    /// Note: you should call set_listener_positions before calling this
    pub fn play_queued_soundeffects(&mut self) {
        let mixer = self.mixer.borrow();
        for s in self.soundeffect_queue.drain(..) {
            mixer.play_soundeffect(s);
        }
    }

    pub fn set_listener_positions<L>(&mut self, listeners: L)
    where
        L: IntoIterator<Item = Vec2>,
    {
        self.mixer.borrow_mut().set_listener_positions(listeners);
    }
}
