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

use anyhow::Result;
use sdl3_sys::keycode::*;

use crate::{
    configfile::GAME_CONFIG,
    fs::find_datafile_path,
    game::{GameControllerSet, MappedKey},
    gfx::Renderer,
    math::Rect,
};

use super::{Image, Texture};

const KEYMAP_OFFSETS: &[u32] = &[
    SDLK_UP,
    SDLK_RIGHT,
    SDLK_DOWN,
    SDLK_LEFT,
    SDLK_A,
    SDLK_B,
    SDLK_C,
    SDLK_D,
    SDLK_E,
    SDLK_F,
    SDLK_G,
    SDLK_H,
    SDLK_I,
    SDLK_J,
    SDLK_K,
    SDLK_L,
    SDLK_M,
    SDLK_N,
    SDLK_O,
    SDLK_P,
    SDLK_Q,
    SDLK_R,
    SDLK_S,
    SDLK_T,
    SDLK_U,
    SDLK_V,
    SDLK_W,
    SDLK_X,
    SDLK_Y,
    SDLK_Z,
    SDLK_RETURN,
    SDLK_0,
    SDLK_1,
    SDLK_2,
    SDLK_3,
    SDLK_4,
    SDLK_5,
    SDLK_6,
    SDLK_7,
    SDLK_8,
    SDLK_9,
    SDLK_PLUS,
    SDLK_MINUS,
    SDLK_PERIOD,
    SDLK_COMMA,
    SDLK_LSHIFT,
    SDLK_LCTRL,
    SDLK_LALT,
];

fn keymap(key: u32) -> Rect {
    let key = match key {
        SDLK_RSHIFT => SDLK_LSHIFT,
        SDLK_RCTRL => SDLK_LCTRL,
        SDLK_RALT => SDLK_LALT,
        SDLK_KP_0 => SDLK_0,
        SDLK_KP_1 => SDLK_1,
        SDLK_KP_2 => SDLK_2,
        SDLK_KP_3 => SDLK_3,
        SDLK_KP_4 => SDLK_4,
        SDLK_KP_5 => SDLK_5,
        SDLK_KP_6 => SDLK_6,
        SDLK_KP_7 => SDLK_7,
        SDLK_KP_8 => SDLK_8,
        SDLK_KP_9 => SDLK_9,
        SDLK_KP_MINUS => SDLK_MINUS,
        SDLK_KP_PLUS => SDLK_PLUS,
        SDLK_KP_ENTER => SDLK_RETURN,
        SDLK_KP_COMMA => SDLK_COMMA,
        SDLK_KP_PERIOD => SDLK_PERIOD,
        x => x,
    };

    for (idx, k) in KEYMAP_OFFSETS.iter().enumerate() {
        if *k == key {
            return Rect::new(idx as i32 * 22, 0, 22, 17);
        }
    }

    Rect::new(0, 0, 0, 0)
}

fn make_keymap_icon(k1: u32, k2: u32, k3: u32, renderer: &Renderer) -> Result<Texture> {
    let mut base = Image::from_file(find_datafile_path(&["images/keys-base.png"])?)?;
    let letters = Image::from_file(find_datafile_path(&["images/keys.png"])?)?;

    letters.blit(keymap(k1), &mut base, (22, 7));
    letters.blit(keymap(k2), &mut base, (5, 33));
    letters.blit(keymap(k3), &mut base, (37, 33));

    Texture::from_image(renderer, &base)
}

fn make_single_key_icon(key: u32, renderer: &Renderer) -> Result<Texture> {
    let mut base = Image::from_file(find_datafile_path(&["images/input-buttons.png"])?)?;
    let letters = Image::from_file(find_datafile_path(&["images/keys.png"])?)?;

    letters.blit(keymap(key), &mut base, (5, 1));

    Texture::from_image(renderer, &base)
}

pub fn make_controller_icon(controller: i32, renderer: &Renderer) -> Result<Texture> {
    assert!(controller > 0);
    if controller > 4 {
        todo!("Gamepad icons");
    }

    let config = GAME_CONFIG.read().unwrap();
    let keymap = match controller {
        1 => &config.keymap1,
        2 => &config.keymap2,
        3 => &config.keymap3,
        4 => &config.keymap4,
        _ => todo!(),
    }
    .as_ref()
    .unwrap_or(&GameControllerSet::DEFAULT_KEYMAP[controller as usize - 1]);

    make_keymap_icon(keymap.thrust, keymap.left, keymap.right, renderer)
}

pub fn make_button_icon(
    controller: i32,
    button: MappedKey,
    renderer: &Renderer,
) -> Result<Texture> {
    assert!(controller > 0);
    if controller > 4 {
        todo!("Gamepad icons");
    }

    let config = GAME_CONFIG.read().unwrap();
    let keymap = match controller {
        1 => &config.keymap1,
        2 => &config.keymap2,
        3 => &config.keymap3,
        4 => &config.keymap4,
        _ => todo!(),
    }
    .as_ref()
    .unwrap_or(&GameControllerSet::DEFAULT_KEYMAP[controller as usize - 1]);

    let key = match button {
        MappedKey::Up => keymap.thrust,
        MappedKey::Right => keymap.right,
        MappedKey::Left => keymap.left,
        MappedKey::Fire1 => keymap.fire_primary,
        MappedKey::Fire2 => keymap.fire_secondary,
    };

    make_single_key_icon(key, renderer)
}
