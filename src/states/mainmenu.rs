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

use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    configfile::{GAME_CONFIG, save_user_config},
    game::{GameControllerSet, MenuButton, PlayerKeymap},
    gfx::{Color, RenderDest, RenderOptions, Renderer, TextureId},
    math::RectF,
    menu::{AnimatedStarfield, Menu, MenuItem, MenuState, MenuValue},
    states::{PlayerSelection, StackableState, StackableStateResult, game_assets::GameAssets},
};

type ActionMenu = Menu<MenuAction>;

pub struct MainMenu {
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,
    assets: Rc<GameAssets>,

    main_menu: ActionMenu,
    options_menu: ActionMenu,
    video_menu: ActionMenu,
    controls_menu: ActionMenu,
    keymap_menu: ActionMenu,
    gameopts_menu: ActionMenu,

    active_keymap: usize,
    background: TextureId,
    logo: TextureId,
    starfield: AnimatedStarfield,

    anim_state: AnimState,
    intro_outro_anim: f32,
}

enum AnimState {
    Intro,
    Normal,
    Outro(StackableStateResult),
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum MenuAction {
    StartGame,
    GoToOptions,
    Quit,
    GoToMain,
    GoToVideoOpts,
    GoToControls,
    GoToKeys1,
    GoToKeys2,
    GoToKeys3,
    GoToKeys4,
    GoToGameOpts,
    ToggleFullscreen,
    KeyMapThrust,
    KeyMapDown,
    KeyMapLeft,
    KeyMapRight,
    KeyMapFire1,
    KeyMapFire2,
    SaveKeyMap,
    SaveVideoOpts,
    ToggleMinimap,
    ToggleBaseRegen,
    SaveGameOpts,
}

impl MainMenu {
    pub fn new(
        assets: Rc<GameAssets>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let r = renderer.borrow();
        let logo = r.texture_store().find_texture("gamelogo")?;
        let logo_h = r.texture_store().get_texture(logo).height() + 32.0;
        let mut main_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Spacer(32.0),
                MenuItem::Link("Start!", MenuAction::StartGame),
                MenuItem::Link("Options", MenuAction::GoToOptions),
                MenuItem::Link("Quit", MenuAction::Quit),
            ],
        )?;

        let options_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Heading("Options", Color::new(0.3, 1.0, 0.3)),
                MenuItem::Spacer(32.0),
                MenuItem::Link("Video", MenuAction::GoToVideoOpts),
                //MenuItem::Link("Audio", 0),
                MenuItem::Link("Game", MenuAction::GoToGameOpts),
                MenuItem::Link("Keyboard controls", MenuAction::GoToControls),
                MenuItem::Spacer(10.0),
                MenuItem::Escape("Back", MenuAction::GoToMain),
            ],
        )?;

        let video_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Heading("Video", Color::new(0.3, 1.0, 0.3)),
                MenuItem::Spacer(32.0),
                MenuItem::Value("Fullscreen: ", MenuAction::ToggleFullscreen),
                MenuItem::Spacer(10.0),
                MenuItem::Link("Save", MenuAction::SaveVideoOpts),
                MenuItem::Escape("Cancel", MenuAction::GoToOptions),
            ],
        )?;

        let controls_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Heading("Controls", Color::new(0.3, 1.0, 0.3)),
                MenuItem::Spacer(32.0),
                MenuItem::Link("Keyboard 1", MenuAction::GoToKeys1),
                MenuItem::Link("Keyboard 2", MenuAction::GoToKeys2),
                MenuItem::Link("Keyboard 3", MenuAction::GoToKeys3),
                MenuItem::Link("Keyboard 4", MenuAction::GoToKeys4),
                MenuItem::Spacer(10.0),
                MenuItem::Escape("Back", MenuAction::GoToOptions),
            ],
        )?;

        let keymap_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Heading("Keymap", Color::new(0.3, 1.0, 0.3)),
                MenuItem::Spacer(32.0),
                MenuItem::Value("Up:", MenuAction::KeyMapThrust),
                MenuItem::Value("Down:", MenuAction::KeyMapDown),
                MenuItem::Value("Left:", MenuAction::KeyMapLeft),
                MenuItem::Value("Right:", MenuAction::KeyMapRight),
                MenuItem::Value("Fire primary:", MenuAction::KeyMapFire1),
                MenuItem::Value("Fire secondary:", MenuAction::KeyMapFire2),
                MenuItem::Spacer(10.0),
                MenuItem::Link("Save", MenuAction::SaveKeyMap),
                MenuItem::Escape("Cancel", MenuAction::GoToControls),
            ],
        )?;

        let gameopts_menu = Menu::new(
            &r,
            &[
                MenuItem::Spacer(logo_h),
                MenuItem::Heading("Game options", Color::new(0.3, 1.0, 0.3)),
                MenuItem::Spacer(32.0),
                MenuItem::Value("Show minimap: ", MenuAction::ToggleMinimap),
                MenuItem::Value("Rebuild bases: ", MenuAction::ToggleBaseRegen),
                MenuItem::Spacer(10.0),
                MenuItem::Link("Save", MenuAction::SaveGameOpts),
                MenuItem::Escape("Cancel", MenuAction::GoToOptions),
            ],
        )?;

        main_menu.appear();

        let background = r.texture_store().find_texture("menubackground")?;
        let starfield = AnimatedStarfield::new(200, r.width() as f32, r.height() as f32);

        drop(r);

        Ok(MainMenu {
            assets,
            renderer,
            controllers,
            main_menu,
            options_menu,
            video_menu,
            controls_menu,
            keymap_menu,
            gameopts_menu,
            active_keymap: 0,
            background,
            logo,
            starfield,
            intro_outro_anim: 1.0,
            anim_state: AnimState::Intro,
        })
    }

    fn init_video_menu(&mut self) {
        let config = GAME_CONFIG.read().unwrap();

        self.video_menu.appear();
        self.video_menu.set_value(
            MenuAction::ToggleFullscreen,
            MenuValue::Toggle(config.video.fullscreen),
        );
    }

    fn save_video_opts(&mut self) {
        let mut config = GAME_CONFIG.read().unwrap().clone();

        config.video.fullscreen = self
            .video_menu
            .get_toggle_value(MenuAction::ToggleFullscreen);

        save_user_config(config);
        self.options_menu.appear();
    }

    fn init_keymap(&mut self, idx: usize) {
        self.active_keymap = idx;
        let config = GAME_CONFIG.read().unwrap();

        let km = match idx {
            0 => &config.keymap1,
            1 => &config.keymap2,
            2 => &config.keymap3,
            3 => &config.keymap4,
            _ => {
                panic!("There are only four keymaps");
            }
        };
        let km = km
            .as_ref()
            .unwrap_or(&GameControllerSet::DEFAULT_KEYMAP[idx]);

        self.keymap_menu
            .set_value(MenuAction::KeyMapThrust, MenuValue::KeyGrab(km.thrust));
        self.keymap_menu
            .set_value(MenuAction::KeyMapDown, MenuValue::KeyGrab(km.down));
        self.keymap_menu
            .set_value(MenuAction::KeyMapLeft, MenuValue::KeyGrab(km.left));
        self.keymap_menu
            .set_value(MenuAction::KeyMapLeft, MenuValue::KeyGrab(km.left));
        self.keymap_menu
            .set_value(MenuAction::KeyMapRight, MenuValue::KeyGrab(km.right));
        self.keymap_menu
            .set_value(MenuAction::KeyMapFire1, MenuValue::KeyGrab(km.fire_primary));
        self.keymap_menu.set_value(
            MenuAction::KeyMapFire2,
            MenuValue::KeyGrab(km.fire_secondary),
        );

        self.keymap_menu.appear();
    }

    fn save_keymap(&mut self) {
        let keymap = Some(PlayerKeymap {
            thrust: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapThrust),
            down: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapDown),
            left: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapLeft),
            right: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapRight),
            fire_primary: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapFire1),
            fire_secondary: self.keymap_menu.get_keygrab_value(MenuAction::KeyMapFire2),
        });

        let mut config = GAME_CONFIG.read().unwrap().clone();

        match self.active_keymap {
            0 => config.keymap1 = keymap,
            1 => config.keymap2 = keymap,
            2 => config.keymap3 = keymap,
            3 => config.keymap4 = keymap,
            _ => panic!("Unhandled keymap"),
        };

        save_user_config(config);
        self.controls_menu.appear();
    }

    fn init_gameopts(&mut self) {
        let config = GAME_CONFIG.read().unwrap();

        self.gameopts_menu.appear();
        self.gameopts_menu.set_value(
            MenuAction::ToggleMinimap,
            MenuValue::Toggle(config.game.minimap),
        );
        self.gameopts_menu.set_value(
            MenuAction::ToggleBaseRegen,
            MenuValue::Toggle(config.game.baseregen),
        );
    }

    fn save_gameopts(&mut self) {
        let mut config = GAME_CONFIG.read().unwrap().clone();

        config.game.minimap = self
            .gameopts_menu
            .get_toggle_value(MenuAction::ToggleMinimap);

        config.game.baseregen = self
            .gameopts_menu
            .get_toggle_value(MenuAction::ToggleBaseRegen);

        save_user_config(config);
        self.options_menu.appear();
    }

    pub fn render(&self) {
        let menus = [
            &self.main_menu,
            &self.options_menu,
            &self.video_menu,
            &self.gameopts_menu,
            &self.controls_menu,
            &self.keymap_menu,
        ];
        let renderer = self.renderer.borrow();
        renderer.clear();

        let bg = renderer.texture_store().get_texture(self.background);

        // Background: starfield
        self.starfield.render(&renderer);

        // Background: top and bottom halves
        let bgoffset = self.intro_outro_anim * self.intro_outro_anim * bg.height() / 2.0;

        let bgrect_dest = RectF::new(
            0.0,
            0.0,
            renderer.width() as f32,
            if bg.width() >= renderer.width() as f32 {
                bg.height() / 2.0
            } else {
                (renderer.width() as f32 / bg.width()) * (bg.height() / 2.0)
            },
        );
        let bgrect_source = RectF::new(
            if bg.width() >= bgrect_dest.w() {
                (bg.width() - bgrect_dest.w()) / 2.0
            } else {
                0.0
            },
            0.0,
            bgrect_dest.w(),
            bg.height() / 2.0,
        );
        let color = Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0 - self.intro_outro_anim * self.intro_outro_anim,
        };
        bg.render(
            &renderer,
            &RenderOptions {
                source: Some(bgrect_source),
                dest: RenderDest::Rect(bgrect_dest.offset(0.0, -bgoffset)),
                color,
                ..Default::default()
            },
        );
        bg.render(
            &renderer,
            &RenderOptions {
                source: Some(bgrect_source.offset(0.0, bg.height() / 2.0)),
                dest: RenderDest::Rect(
                    bgrect_dest.offset(0.0, renderer.height() as f32 - bgrect_dest.h() + bgoffset),
                ),
                color,
                ..Default::default()
            },
        );

        // Logo
        let logo = renderer.texture_store().get_texture(self.logo);
        logo.render(
            &renderer,
            &RenderOptions {
                dest: RenderDest::Rect(RectF::new(
                    (renderer.width() as f32 - logo.width()) / 2.0,
                    (renderer.height() as f32 - self.controls_menu.height()) / 2.0, // tallest menu height
                    logo.width(),
                    logo.height(),
                )),
                color,
                ..Default::default()
            },
        );

        // Menu
        for menu in menus {
            if menu.state() != MenuState::Disappeared {
                menu.render(&renderer);
            }
        }
        renderer.present();
    }
}

impl StackableState for MainMenu {
    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        match retval.downcast::<AnimatedStarfield>() {
            Ok(s) => {
                self.starfield = *s;
                StackableStateResult::Continue
            }
            Err(e) => StackableStateResult::Error(anyhow!(
                "Main menu state received unexpected return value type {:?}",
                e.type_id()
            )),
        }
    }

    fn resize_screen(&mut self) {
        let menus = [
            &mut self.main_menu,
            &mut self.options_menu,
            &mut self.video_menu,
            &mut self.gameopts_menu,
            &mut self.controls_menu,
            &mut self.keymap_menu,
        ];
        let size = self.renderer.borrow().size();
        for menu in menus {
            menu.update_window_size(size.0 as f32, size.1 as f32);
        }
        self.starfield.update_screensize(size);
    }

    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        let menus = [
            &mut self.main_menu,
            &mut self.options_menu,
            &mut self.video_menu,
            &mut self.gameopts_menu,
            &mut self.controls_menu,
            &mut self.keymap_menu,
        ];

        let action = menus
            .into_iter()
            .find(|m| m.state() == MenuState::Normal)
            .and_then(|m| {
                let action = m.handle_button(button);
                if action.is_some() {
                    m.disappear();
                }
                action
            });

        if let Some(action) = action {
            match action {
                MenuAction::StartGame => {
                    self.intro_outro_anim = 0.0;
                    self.anim_state = AnimState::Outro(StackableStateResult::Push(Box::new(
                        PlayerSelection::new(
                            self.assets.clone(),
                            Rc::new(RefCell::new(self.starfield.clone())),
                            self.controllers.clone(),
                            self.renderer.clone(),
                        ),
                    )))
                }
                MenuAction::GoToOptions => self.options_menu.appear(),
                MenuAction::GoToMain => self.main_menu.appear(),
                MenuAction::GoToVideoOpts => self.init_video_menu(),
                MenuAction::GoToControls => self.controls_menu.appear(),
                MenuAction::GoToGameOpts => self.init_gameopts(),
                MenuAction::GoToKeys1 => self.init_keymap(0),
                MenuAction::GoToKeys2 => self.init_keymap(1),
                MenuAction::GoToKeys3 => self.init_keymap(2),
                MenuAction::GoToKeys4 => self.init_keymap(3),
                MenuAction::SaveKeyMap => self.save_keymap(),
                MenuAction::SaveVideoOpts => self.save_video_opts(),
                MenuAction::SaveGameOpts => self.save_gameopts(),
                MenuAction::Quit => {
                    self.intro_outro_anim = 0.0;
                    self.anim_state = AnimState::Outro(StackableStateResult::Pop)
                }
                _ => {}
            }
        }

        StackableStateResult::Continue
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        let menus = [
            &mut self.main_menu,
            &mut self.options_menu,
            &mut self.video_menu,
            &mut self.gameopts_menu,
            &mut self.controls_menu,
            &mut self.keymap_menu,
        ];

        for menu in menus {
            match menu.state() {
                MenuState::Appearing | MenuState::Normal => {
                    menu.step(Some(&self.controllers.borrow()), timestep);
                }
                MenuState::Disappearing => {
                    menu.step(Some(&self.controllers.borrow()), timestep);
                }
                MenuState::Disappeared => {}
            }
        }

        match self.anim_state {
            AnimState::Normal => {}
            AnimState::Intro => self.intro_outro_anim -= timestep,
            AnimState::Outro(_) => self.intro_outro_anim += timestep,
        };

        // Note: star animation is not updated here so we get a static starfield.
        // Updates start in the next state, giving us a nice warp effect.
        //self.starfield.step(timestep);

        self.render();

        match self.anim_state {
            AnimState::Intro => {
                if self.intro_outro_anim <= 0.0 {
                    self.intro_outro_anim = 0.0;
                    self.anim_state = AnimState::Normal;
                }
            }
            AnimState::Normal => {}
            AnimState::Outro(_) => {
                if self.intro_outro_anim > 1.0 {
                    self.intro_outro_anim = 1.0;
                    let ret = std::mem::replace(&mut self.anim_state, AnimState::Intro);
                    self.main_menu.appear();

                    return match ret {
                        AnimState::Outro(ret) => ret,
                        _ => unreachable!(),
                    };
                }
            }
        }

        StackableStateResult::Continue
    }
}
