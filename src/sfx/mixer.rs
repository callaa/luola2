use std::{
    pin::Pin,
    ptr::{null, null_mut},
    rc::Rc,
};

use core::ffi::c_void;
use smallvec::SmallVec;

use sdl3_mixer_sys::mixer::{
    MIX_Audio, MIX_AudioMSToFrames, MIX_CreateMixerDevice, MIX_CreateTrack, MIX_DestroyAudio,
    MIX_DestroyMixer, MIX_GetTrackAudio, MIX_Mixer, MIX_PROP_PLAY_FADE_IN_MILLISECONDS_NUMBER,
    MIX_PlayTrack, MIX_SetTrackAudio, MIX_SetTrackFrequencyRatio, MIX_SetTrackGain,
    MIX_SetTrackStereo, MIX_SetTrackStoppedCallback, MIX_StereoGains, MIX_StopTrack, MIX_Track,
    MIX_TrackPlaying,
};
use sdl3_sys::{
    audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK,
    properties::{SDL_CreateProperties, SDL_PropertiesID, SDL_SetNumberProperty},
};

use crate::{gfx::SdlError, math::Vec2};

/// Abstraction for SDL mixer
pub struct Mixer {
    // mixer instance may be null if sounds are not initialized
    pub(super) mixer: *mut MIX_Mixer,

    listeners: Vec<Vec2>,

    sounds_enabled: bool,
    music_enabled: bool,

    blips: TrackPool<4>,
    explosions: TrackPool<12>,
    weapons: TrackPool<4>,

    music: Pin<Box<Playlist>>,
}

struct AudioWrapper(pub(super) *mut MIX_Audio);

// Note: MIX_Audio is internally reference counted, but this refcount is not exposed in the public API
// If this changes in some future version, we can use it directly.
#[derive(Clone)]
pub struct Audio(Rc<AudioWrapper>);

#[derive(Clone)]
pub enum PlayableSoundEffect {
    /**
     * UI blips etc. Not positional.
     */
    Blip(Audio),

    /**
     * Explosions, non-player weapons fire, and other environmental sounds
     */
    Explosion(Audio, Vec2, FrequencyRatio),

    /**
     * Player weapon fire. (Not positional, always centered on player)
     */
    Weapon(Audio, FrequencyRatio),
}

struct Playlist {
    mixer: *mut MIX_Mixer, // same mixer instance as owning Mixer
    playlist: Vec<Audio>,
    playlist_next: usize,
    fadein_props: SDL_PropertiesID,

    // two music tracks for crossfading
    music1: *mut MIX_Track,
    music2: *mut MIX_Track,

    // Callbacks refer to this object so it must not move
    _pin: std::marker::PhantomPinned,
}

pub type FrequencyRatio = f32;

struct TrackPool<const N: usize>([*mut MIX_Track; N]);

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

        let music = Box::pin(Playlist::new(mixer));

        Self {
            mixer,
            listeners: Vec::new(),
            blips,
            explosions,
            weapons,
            music,
            sounds_enabled: true,
            music_enabled: true,
        }
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.sounds_enabled = volume > 0.01;
        self.blips.set_volume(volume);
        self.explosions.set_volume(volume);
        self.weapons.set_volume(volume);
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_enabled = volume > 0.01;
        self.music.set_volume(volume.clamp(0.0, 1.0));
    }

    // Stop all music playback
    pub fn stop_music(&mut self, fadeout: i64) {
        self.music.as_mut().stop(fadeout);
    }

    // Replace current playlist with a music file that plays only once
    pub fn play_music_single(&mut self, audio: &Audio) {
        if self.music_enabled {
            self.music.as_mut().play_single(audio);
        } else {
            self.stop_music(0);
        }
    }

    // Replace current playlist with a new looping playlist
    pub fn play_music_loop(&mut self, playlist: Vec<Audio>) {
        if self.music_enabled {
            self.music.as_mut().play_loop(playlist);
        } else {
            self.stop_music(0);
        }
    }

    pub fn play_soundeffects(&self, sounds: &mut Vec<PlayableSoundEffect>) {
        if self.mixer.is_null() || !self.sounds_enabled {
            return;
        }

        // Explosion effects are positional and they typically come in bursts.
        // Merge together explosion sounds so we don't use up as many tracks
        struct QueuedPositionalSound {
            audio: Audio,
            gains: MIX_StereoGains,
            fr: FrequencyRatio,
            count: f32,
        }

        let mut positional_queue = SmallVec::<[QueuedPositionalSound; 4]>::new();

        for sound in sounds.drain(..) {
            match sound {
                PlayableSoundEffect::Blip(audio) => {
                    self.blips.play_soundeffect(audio.ptr(), None, 1.0);
                }
                PlayableSoundEffect::Explosion(audio, pos, fr) => {
                    if let Some(gains) = self.map_stereo_gains(pos) {
                        if let Some(merge) =
                            positional_queue.iter_mut().find(|qps| qps.audio == audio)
                        {
                            merge.gains.left += gains.left;
                            merge.gains.right += gains.right;
                            merge.fr += fr;
                            merge.count += 1.0;
                        } else {
                            positional_queue.push(QueuedPositionalSound {
                                audio,
                                gains,
                                fr,
                                count: 1.0,
                            });
                        }
                    }
                }
                PlayableSoundEffect::Weapon(audio, frequency_ratio) => {
                    self.weapons
                        .play_soundeffect(audio.ptr(), None, frequency_ratio);
                }
            }
        }

        for pqs in positional_queue {
            self.explosions
                .play_soundeffect(pqs.audio.ptr(), Some(pqs.gains), pqs.fr / pqs.count);
        }
    }

    // Immediately play a sound effect
    pub fn play_soundeffect(&self, sound: PlayableSoundEffect) -> bool {
        if self.mixer.is_null() || !self.sounds_enabled {
            return true;
        }

        match sound {
            PlayableSoundEffect::Blip(audio) => self.blips.play_soundeffect(audio.ptr(), None, 1.0),
            PlayableSoundEffect::Explosion(audio, pos, frequency_ratio) => {
                self.map_stereo_gains(pos).is_none_or(|gains| {
                    self.explosions
                        .play_soundeffect(audio.ptr(), Some(gains), frequency_ratio)
                })
            }

            PlayableSoundEffect::Weapon(audio, frequency_ratio) => {
                self.weapons
                    .play_soundeffect(audio.ptr(), None, frequency_ratio)
            }
        }
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

impl Playlist {
    fn new(mixer: *mut MIX_Mixer) -> Self {
        let music1 = unsafe { MIX_CreateTrack(mixer) };
        let music2 = unsafe { MIX_CreateTrack(mixer) };
        let fadein_props = unsafe { SDL_CreateProperties() };

        Self {
            mixer,
            music1,
            music2,
            playlist: Vec::new(),
            playlist_next: 0,
            fadein_props,
            _pin: std::marker::PhantomPinned,
        }
    }

    fn set_volume(&self, volume: f32) {
        unsafe {
            MIX_SetTrackGain(self.music1, volume);
            MIX_SetTrackGain(self.music2, volume);
        }
    }

    fn play_music(&mut self, audio: &Audio, crossfade: i64, use_callback: bool) {
        let playing1 = unsafe { MIX_TrackPlaying(self.music1) };
        let playing2 = unsafe { MIX_TrackPlaying(self.music2) };

        let mut stop_track: *mut MIX_Track = null_mut();
        let play_track: *mut MIX_Track;

        if playing1 && playing2 {
            // TODO pick older track
            stop_track = self.music2;
            play_track = self.music1;
        } else if playing1 {
            stop_track = self.music1;
            play_track = self.music2;
        } else {
            if playing2 {
                stop_track = self.music2;
            }
            play_track = self.music1;
        }

        if !stop_track.is_null() {
            unsafe {
                let stop_audio = MIX_GetTrackAudio(stop_track);
                if audio.same_as(stop_audio) {
                    // Don't restart a running track
                    return;
                }

                MIX_SetTrackStoppedCallback(stop_track, None, null_mut());
                MIX_StopTrack(stop_track, MIX_AudioMSToFrames(stop_audio, crossfade));
            }
        }

        unsafe {
            MIX_SetTrackAudio(play_track, audio.ptr());
            SDL_SetNumberProperty(
                self.fadein_props,
                MIX_PROP_PLAY_FADE_IN_MILLISECONDS_NUMBER,
                if stop_track.is_null() { 0 } else { crossfade },
            );
            MIX_PlayTrack(play_track, self.fadein_props);
            if use_callback {
                MIX_SetTrackStoppedCallback(
                    play_track,
                    Some(playlist_track_finished_callback),
                    self as *mut Self as *mut c_void,
                );
            }
        }
    }

    pub fn stop(self: Pin<&mut Self>, fadeout: i64) {
        let this = unsafe { self.get_unchecked_mut() };
        this.playlist.clear();

        unsafe {
            let audio = MIX_GetTrackAudio(this.music1);
            if !audio.is_null() {
                MIX_StopTrack(this.music1, MIX_AudioMSToFrames(audio, fadeout));
            }

            let audio = MIX_GetTrackAudio(this.music2);
            if !audio.is_null() {
                MIX_StopTrack(this.music2, MIX_AudioMSToFrames(audio, fadeout));
            }
        }
    }

    pub fn play_single(self: Pin<&mut Self>, music: &Audio) {
        unsafe { self.get_unchecked_mut() }.play_music(music, 1000, false);
    }

    pub fn play_loop(self: Pin<&mut Self>, playlist: Vec<Audio>) {
        if playlist.is_empty() {
            self.stop(1000);
        } else {
            let this = unsafe { self.get_unchecked_mut() };
            this.play_music(&playlist[0], 1000, true);
            this.playlist_next = 1 % playlist.len();
            this.playlist = playlist;
        }
    }
}

// Track finished callback. The reason Playlist must be Pinned.
extern "C" fn playlist_track_finished_callback(userdata: *mut c_void, _track: *mut MIX_Track) {
    let pl = unsafe { (userdata as *mut Playlist).as_mut() }
        .expect("callback userdata should have been set");
    let next = pl.playlist_next;
    if next < pl.playlist.len() {
        let audio = pl.playlist[next].clone();
        pl.play_music(&audio, 1000, true);
        pl.playlist_next = (next + 1) % pl.playlist.len();
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

    fn set_volume(&self, volume: f32) {
        for t in self.0 {
            unsafe {
                MIX_SetTrackGain(t, volume);
            }
        }
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

impl Audio {
    pub(super) fn new(audio: *mut MIX_Audio) -> Self {
        Self(Rc::new(AudioWrapper(audio)))
    }

    pub fn invalid() -> Self {
        Self(Rc::new(AudioWrapper(null_mut())))
    }

    pub fn is_valid(&self) -> bool {
        !self.0.0.is_null()
    }

    fn same_as(&self, audio: *mut MIX_Audio) -> bool {
        self.ptr() == audio
    }

    fn ptr(&self) -> *mut MIX_Audio {
        self.0.0
    }
}

impl PartialEq for Audio {
    fn eq(&self, other: &Self) -> bool {
        self.ptr() == other.ptr()
    }
}

impl Drop for AudioWrapper {
    fn drop(&mut self) {
        unsafe {
            MIX_DestroyAudio(self.0);
        }
    }
}

impl mlua::UserData for Audio {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("is_valid", |_, this| Ok(this.is_valid()));
    }
}

impl mlua::FromLua for Audio {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Audio".to_owned(),
                message: Some("expected Audio".to_string()),
            }),
        }
    }
}
