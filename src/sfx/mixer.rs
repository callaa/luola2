use std::{
    cell::RefCell,
    collections::HashMap,
    os::unix::ffi::OsStringExt,
    ptr::{null, null_mut},
    rc::Rc,
    u32,
};

use mlua::{Lua, LuaString};
use sdl3_mixer_sys::mixer::{
    MIX_Audio, MIX_CreateMixerDevice, MIX_CreateTrack, MIX_DestroyAudio, MIX_DestroyMixer,
    MIX_LoadAudio, MIX_Mixer, MIX_PlayTrack, MIX_SetTrackAudio, MIX_SetTrackFrequencyRatio,
    MIX_SetTrackStereo, MIX_StereoGains, MIX_Track, MIX_TrackPlaying,
};
use sdl3_sys::{audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, properties::SDL_PropertiesID};

use crate::{
    fs::{glob_datafiles, pathbuf_to_cstring},
    gfx::SdlError,
    math::Vec2,
};

#[derive(Copy, Clone, PartialEq)]
pub struct SoundEffectId(u32);
struct SoundEffect(*mut MIX_Audio);

pub struct Mixer {
    // mixer instance may be null if sounds couldn't be initialized
    mixer: *mut MIX_Mixer,

    samples: Vec<SoundEffect>,
    sample_map: HashMap<Vec<u8>, SoundEffectId>,

    listeners: Vec<Vec2>,
    soundeffect_queue: Vec<PlayableSoundEffect>,

    blips: TrackPool<4>,
    explosions: TrackPool<12>,
    weapons: TrackPool<4>,
}

pub type FrequencyRatio = f32;

#[derive(Clone, Copy)]
pub enum PlayableSoundEffect {
    /**
     * UI blips etc. Not positional.
     */
    Blip(SoundEffectId), // not queued

    /**
     * Explosions, non-player weapons fire, and other environmental sounds
     */
    Explosion(SoundEffectId, Vec2, FrequencyRatio),

    /**
     * Player weapon fire. (Not positional, always centered on player)
     */
    Weapon(SoundEffectId, FrequencyRatio),
}

impl PlayableSoundEffect {
    pub fn id(&self) -> SoundEffectId {
        match self {
            Self::Blip(id) => *id,
            Self::Explosion(id, _, _) => *id,
            Self::Weapon(id, _) => *id,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.id().is_valid()
    }
}

struct TrackPool<const N: usize>([*mut MIX_Track; N]);

fn randomize_frequency_ratio(randomization: f32) -> f32 {
    let r = (fastrand::f32() - 0.5) * randomization;
    if r <= 0.0 { 1.0 / (-r + 1.0) } else { 1.0 + r }
}

impl Mixer {
    pub fn new(enabled: bool) -> Self {
        let mixer = if enabled {
            let mixer = unsafe { MIX_CreateMixerDevice(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, null()) };
            if mixer.is_null() {
                SdlError::log("Couldn't create mixer device");
            }
            mixer
        } else {
            log::info!("Audio not enabled");
            null_mut()
        };

        let blips = TrackPool::new(mixer);
        let explosions = TrackPool::new(mixer);
        let weapons = TrackPool::new(mixer);

        Self {
            mixer,
            samples: Vec::new(),
            sample_map: HashMap::new(),
            listeners: Vec::new(),
            soundeffect_queue: Vec::new(),
            blips,
            explosions,
            weapons,
        }
    }

    pub fn make_lua_api(lua: &Lua, mixer: Rc<RefCell<Mixer>>) -> mlua::Result<mlua::Table> {
        let api = lua.create_table()?;

        // Function for finding a audio sample handle
        {
            let mixer = mixer.clone();
            api.set(
                "get",
                lua.create_function(move |_, name: LuaString| {
                    let m = mixer.borrow();
                    Ok(m.find_soundeffect(&name.as_bytes()))
                })?,
            )?;
        }

        // Playback functions for all track types
        {
            let mixer = mixer.clone();
            api.set(
                "blip",
                lua.create_function(move |_, id: SoundEffectId| {
                    mixer
                        .borrow()
                        .play_soundeffect(PlayableSoundEffect::Blip(id));
                    Ok(())
                })?,
            )?;
        }

        {
            let mixer = mixer.clone();
            api.set(
                "explosion",
                lua.create_function(
                    move |_, (id, pos, fr): (SoundEffectId, Vec2, Option<f32>)| {
                        mixer
                            .borrow_mut()
                            .enqueue_soundeffect(PlayableSoundEffect::Explosion(
                                id,
                                pos,
                                fr.map_or(1.0, randomize_frequency_ratio),
                            ));
                        Ok(())
                    },
                )?,
            )?;
        }
        api.set(
            "weapon",
            lua.create_function(move |_, (id, fr): (SoundEffectId, Option<f32>)| {
                mixer
                    .borrow_mut()
                    .enqueue_soundeffect(PlayableSoundEffect::Weapon(
                        id,
                        fr.map_or(1.0, randomize_frequency_ratio),
                    ));
                Ok(())
            })?,
        )?;
        Ok(api)
    }

    pub fn load_sound_effects(&mut self) {
        if self.mixer.is_null() {
            log::info!("SDL Mixer not initialized: not loading sound effects.");
            return;
        }

        let files = match glob_datafiles("sounds", "*.ogg") {
            Ok(f) => f,
            Err(e) => {
                log::error!("Couldn't get sound files: {}", e);
                return;
            }
        };

        for file in files {
            let name = file
                .as_path()
                .file_stem()
                .expect("valid filename")
                .to_owned();
            let pathstr = pathbuf_to_cstring(file).expect("valid path");
            let audio = unsafe { MIX_LoadAudio(self.mixer, pathstr.as_ptr(), true) };

            if audio.is_null() {
                SdlError::log("Couldn't load file");
            } else {
                log::debug!("Loaded sound effect: {:?} (#{})", name, self.samples.len());
                self.sample_map
                    .insert(name.into_vec(), SoundEffectId(self.samples.len() as u32));
                self.samples.push(SoundEffect(audio));
            }
        }

        if self.samples.is_empty() {
            log::warn!("No sound effects found!");
        }
    }

    /**
     * Find a handle for the given sound effect.
     *
     * Name is given as a byte slice for efficient use via Lua API.
     * Returns an invalid SoundEffectId if sample wasn't found.
     */
    pub fn find_soundeffect(&self, name: &[u8]) -> SoundEffectId {
        self.sample_map.get(name).copied().unwrap_or_else(|| {
            if !self.samples.is_empty() {
                // don't bother spamming warnings if sounds are disabled
                log::warn!(
                    "Sound effect \"{}\" not found!",
                    str::from_utf8(name).unwrap()
                );
            }
            SoundEffectId::invalid()
        })
    }

    /// Shorthand function for playing UI blips
    pub fn play_blip(&self, id: SoundEffectId) {
        self.play_soundeffect(PlayableSoundEffect::Blip(id));
    }

    // Immediately play a sound effect
    pub fn play_soundeffect(&self, sound: PlayableSoundEffect) -> bool {
        if self.mixer.is_null() || !sound.is_valid() {
            return true;
        }

        let audio = self.samples[sound.id().0 as usize].0;

        match sound {
            PlayableSoundEffect::Blip(_) => self.blips.play_soundeffect(audio, None, 1.0),
            PlayableSoundEffect::Explosion(_, pos, frequency_ratio) => {
                self.map_stereo_gains(pos).map_or(true, |gains| {
                    self.explosions
                        .play_soundeffect(audio, Some(gains), frequency_ratio)
                })
            }

            PlayableSoundEffect::Weapon(_, frequency_ratio) => {
                self.weapons.play_soundeffect(audio, None, frequency_ratio)
            }
        }
    }

    // Add a sound effect to the playback queue
    pub fn enqueue_soundeffect(&mut self, sound: PlayableSoundEffect) {
        if self.mixer.is_null() || !sound.is_valid() {
            return;
        }

        self.soundeffect_queue.push(sound);
    }

    /// Play as many queued sound effects as we have space for in the mixer.
    /// Note: you should call set_listener_positions before calling this
    pub fn play_queued_soundeffects(&mut self) {
        for s in &self.soundeffect_queue {
            self.play_soundeffect(*s);
        }
        self.soundeffect_queue.clear();
    }

    fn map_stereo_gains(&self, pos: Vec2) -> Option<MIX_StereoGains> {
        if let Some(nearest) = self.nearest_listener(pos) {
            let d = pos - nearest;

            // Distance from which the sound can be heard
            const POWER: f32 = 800.0;

            // Volume falls off with square of distance
            let mm = d.magnitude_squared();
            let loudness = -mm / (POWER * POWER) + 1.0;

            if loudness <= 0.0 {
                return None;
            }

            // Ratio of sound field overlapping the listener on the horizontal axis
            let ratio = ((d.0 + POWER) / (POWER * 2.0)).clamp(0.0, 1.0);

            /*
            log::info!(
                "Mapping {}, d/P={}, loudness={}, ratio={}",
                d,
                d.0 / POWER,
                loudness,
                ratio,
            );
            */

            Some(MIX_StereoGains {
                left: loudness * (1.0 - ratio),
                right: loudness * ratio,
            })
        } else {
            log::debug!("No listeners!");
            None
        }
    }

    fn nearest_listener(&self, pos: Vec2) -> Option<Vec2> {
        let mut nearest = None;
        let mut nearest_dd = f32::MAX;
        for l in &self.listeners {
            let dd = l.dist_squared(pos);
            if dd < nearest_dd {
                nearest = Some(*l);
                nearest_dd = dd;
            }
        }
        nearest
    }

    pub fn set_listener_positions<L>(&mut self, listeners: L)
    where
        L: IntoIterator<Item = Vec2>,
    {
        self.listeners.clear();
        self.listeners.extend(listeners);
    }
}

impl SoundEffectId {
    fn invalid() -> Self {
        Self(u32::MAX)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

impl<const N: usize> TrackPool<N> {
    fn new(mixer: *mut MIX_Mixer) -> Self {
        let mut tracks = [null_mut(); N];

        if !mixer.is_null() {
            for t in tracks.iter_mut() {
                *t = unsafe { MIX_CreateTrack(mixer) };
                if (*t).is_null() {
                    SdlError::log("Couldn't create mixer track");
                    break;
                }
            }
        }

        Self(tracks)
    }

    /// Play a sound effect. Returns false if no track was available
    fn play_soundeffect(
        &self,
        audio: *mut MIX_Audio,
        gains: Option<MIX_StereoGains>,
        frequency_ratio: FrequencyRatio,
    ) -> bool {
        for track in self.0 {
            unsafe {
                if !track.is_null() && !MIX_TrackPlaying(track) {
                    MIX_SetTrackAudio(track, audio);
                    if let Some(gains) = gains {
                        MIX_SetTrackStereo(track, &gains);
                    }
                    MIX_SetTrackFrequencyRatio(track, frequency_ratio);
                    MIX_PlayTrack(track, SDL_PropertiesID(0));
                    return true;
                }
            }
        }
        false
    }
}

impl Drop for Mixer {
    fn drop(&mut self) {
        unsafe {
            MIX_DestroyMixer(self.mixer);
        }
    }
}

impl Drop for SoundEffect {
    fn drop(&mut self) {
        unsafe {
            MIX_DestroyAudio(self.0);
        }
    }
}

impl mlua::UserData for SoundEffectId {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("is_valid", |_, this| Ok(this.is_valid()));
    }
}

impl mlua::FromLua for SoundEffectId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "SoundEffectId".to_owned(),
                message: Some("expected SoundEffectId".to_string()),
            }),
        }
    }
}
