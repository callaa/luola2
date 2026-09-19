use super::mixer::{Audio, Mixer};
use crate::fs::{glob_datafiles, pathbuf_to_cstring};
use crate::gfx::SdlError;
use std::collections::HashMap;
use std::os::unix::ffi::OsStringExt;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use sdl3_mixer_sys::mixer::MIX_LoadAudio;

#[derive(Clone)]
pub struct MusicStore {
    mixer: Rc<RefCell<Mixer>>,
    music: HashMap<Vec<u8>, Audio>,
}

impl MusicStore {
    pub fn new(mixer: Rc<RefCell<Mixer>>) -> Self {
        Self {
            mixer,
            music: HashMap::new(),
        }
    }

    pub fn load_bundled_music(mut self) -> Self {
        let mixer = self.mixer.borrow().mixer;
        if mixer.is_null() {
            log::info!("SDL Mixer not initialized: not loading sound effects.");
            return self;
        }

        let files = match glob_datafiles("music", "*.ogg") {
            Ok(f) => f,
            Err(e) => {
                log::error!("Couldn't get music files: {}", e);
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
            let audio = unsafe { MIX_LoadAudio(mixer, pathstr.as_ptr(), false) };

            if audio.is_null() {
                SdlError::log("Couldn't load file");
            } else {
                self.music.insert(name.into_vec(), Audio::new(audio));
            }
        }

        if self.music.is_empty() {
            log::warn!("No music found!");
        }

        self
    }

    pub fn load_extra_music<T: AsRef<str>>(&mut self, root: &Path, filenames: &[T]) {
        let mixer = self.mixer.borrow().mixer;
        for filename in filenames {
            let path: PathBuf = [root, Path::new(filename.as_ref())].iter().collect();
            let filestem = path
                .file_stem()
                .expect("non-empty filename")
                .to_os_string()
                .into_vec();
            let pathstr = match pathbuf_to_cstring(path) {
                Ok(p) => p,
                Err(err) => {
                    log::error!("Invalid path {}: {}", filename.as_ref(), err);
                    continue;
                }
            };

            let audio = unsafe { MIX_LoadAudio(mixer, pathstr.as_ptr(), false) };

            if audio.is_null() {
                SdlError::log(&format!("Couldn't load music {}", filename.as_ref()));
            } else {
                log::debug!("Loaded extra music: {:?}", filename.as_ref());
                self.music.insert(filestem, Audio::new(audio));
            }
        }
    }

    pub fn get(&self, name: &[u8]) -> Option<Audio> {
        self.music.get(name).cloned()
    }

    pub fn get_playlist<T: AsRef<[u8]>>(&self, names: &[T]) -> Vec<Audio> {
        let mut playlist = Vec::new();

        for name in names {
            if let Some(audio) = self.get(name.as_ref()) {
                playlist.push(audio);
            } else {
                log::warn!("Music {:?} not found!", str::from_utf8(name.as_ref()))
            }
        }
        playlist
    }

    pub fn play_playlist<T: AsRef<[u8]>>(&self, names: &[T]) {
        self.mixer
            .borrow_mut()
            .play_music_loop(self.get_playlist(names));
    }
}
