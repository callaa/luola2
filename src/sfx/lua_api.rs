use crate::{
    math::Vec2,
    sfx::{MusicStore, SfxStore},
};

use super::{Audio, PlayableSoundEffect};
use mlua::{Lua, LuaString};
use std::{cell::RefCell, rc::Rc};

fn randomize_frequency_ratio(randomization: f32) -> f32 {
    let r = (fastrand::f32() - 0.5) * randomization;
    if r <= 0.0 { 1.0 / (-r + 1.0) } else { 1.0 + r }
}

pub fn make_lua_sfx_api(
    lua: &Lua,
    sfx: Rc<RefCell<SfxStore>>,
    music: Option<Rc<RefCell<MusicStore>>>,
) -> mlua::Result<mlua::Table> {
    let api = lua.create_table()?;

    // Set volumes
    {
        let mixer = sfx.borrow().get_mixer();
        let mixer2 = mixer.clone();

        api.set(
            "set_sfx_volume",
            lua.create_function(move |_, vol: f32| Ok(mixer.borrow_mut().set_sfx_volume(vol)))?,
        )?;

        api.set(
            "set_music_volume",
            lua.create_function(move |_, vol: f32| Ok(mixer2.borrow_mut().set_music_volume(vol)))?,
        )?;
    }

    // Function for finding an audio sample
    {
        let sfx = sfx.clone();
        api.set(
            "get",
            lua.create_function(move |_, name: LuaString| {
                let s = sfx.borrow();
                Ok(s.find_soundeffect(&name.as_bytes()))
            })?,
        )?;
    }

    // Playback functions for all track types
    {
        let sfx = sfx.clone();
        api.set(
            "blip",
            lua.create_function(move |_, audio: Option<Audio>| {
                if let Some(audio) = audio {
                    sfx.borrow()
                        .play_soundeffect(PlayableSoundEffect::Blip(audio));
                }
                Ok(())
            })?,
        )?;
    }

    {
        let sfx = sfx.clone();
        api.set(
            "explosion",
            lua.create_function(
                move |_, (audio, pos, fr): (Option<Audio>, Vec2, Option<f32>)| {
                    if let Some(audio) = audio {
                        sfx.borrow_mut()
                            .enqueue_soundeffect(PlayableSoundEffect::Explosion(
                                audio,
                                pos,
                                fr.map_or(1.0, randomize_frequency_ratio),
                            ));
                    }
                    Ok(())
                },
            )?,
        )?;
    }
    api.set(
        "weapon",
        lua.create_function(move |_, (audio, fr): (Option<Audio>, Option<f32>)| {
            if let Some(audio) = audio {
                sfx.borrow_mut()
                    .enqueue_soundeffect(PlayableSoundEffect::Weapon(
                        audio,
                        fr.map_or(1.0, randomize_frequency_ratio),
                    ));
            }
            Ok(())
        })?,
    )?;

    // Music playback
    if let Some(music) = music {
        api.set(
            "music_loop",
            lua.create_function(move |_, playlist: Vec<String>| {
                music.borrow_mut().play_playlist(&playlist);
                Ok(())
            })?,
        )?;
    }
    Ok(api)
}
