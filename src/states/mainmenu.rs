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
    demos::AnimatedStarfield,
    game::{GameControllerSet, MenuButton},
    gfx::{Color, RenderDest, RenderOptions, Renderer, TextureId},
    math::RectF,
    menu::LuaMenu,
    states::{PlayerSelection, StackableState, StackableStateResult, game_assets::GameAssets},
};

pub struct MainMenu {
    controllers: Rc<RefCell<GameControllerSet>>,
    renderer: Rc<RefCell<Renderer>>,
    assets: Rc<GameAssets>,

    luamenu: LuaMenu,

    background: TextureId,
    starfield: Rc<RefCell<AnimatedStarfield>>,

    anim_state: AnimState,
    intro_outro_anim: f32,
}

enum AnimState {
    Intro,
    Normal,
    Outro(StackableStateResult),
}

impl MainMenu {
    pub fn new(
        assets: Rc<GameAssets>,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let luamenu = LuaMenu::new(
            "menus.menu",
            renderer.clone(),
            RectF::new(
                0.0,
                0.0,
                renderer.borrow().width() as f32,
                renderer.borrow().height() as f32,
            ),
        )?;

        let r = renderer.borrow();
        let background = r.texture_store().find_texture(b"menubackground")?;
        let starfield = Rc::new(RefCell::new(Self::new_starfield(r.size())));

        drop(r);

        Ok(MainMenu {
            assets,
            renderer,
            controllers,
            luamenu,
            background,
            starfield,
            intro_outro_anim: 1.0,
            anim_state: AnimState::Intro,
        })
    }

    fn new_starfield(screen_size: (i32, i32)) -> AnimatedStarfield {
        AnimatedStarfield::new(200, screen_size.0 as f32, screen_size.1 as f32)
    }

    pub fn render(&self) {
        let renderer = self.renderer.borrow();
        renderer.clear();

        let bg = renderer.texture_store().get_texture(self.background);

        // Background: starfield
        self.starfield.borrow().render(&renderer);

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

        // Menu
        self.luamenu.render();

        renderer.present();
    }
}

impl StackableState for MainMenu {
    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        match retval.downcast::<AnimatedStarfield>() {
            Ok(s) => {
                self.starfield.replace(*s);
                StackableStateResult::Continue
            }
            Err(e) => StackableStateResult::Error(anyhow!(
                "Main menu state received unexpected return value type {:?}",
                e.type_id()
            )),
        }
    }

    fn resize_screen(&mut self) {
        let size = self.renderer.borrow().size();
        self.luamenu
            .relayout(RectF::new(0.0, 0.0, size.0 as f32, size.1 as f32));
        self.starfield.borrow_mut().update_screensize(size);
    }

    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        let result = match self.luamenu.handle_button(button) {
            Ok(res) => res,
            Err(e) => {
                return StackableStateResult::Error(e);
            }
        };

        match result.as_str() {
            "" => {}
            "start" => {
                self.intro_outro_anim = 0.0;
                self.anim_state =
                    AnimState::Outro(StackableStateResult::Push(Box::new(PlayerSelection::new(
                        self.assets.clone(),
                        self.starfield.clone(),
                        self.controllers.clone(),
                        self.renderer.clone(),
                    ))))
            }
            "quit" => {
                self.intro_outro_anim = 0.0;
                self.anim_state = AnimState::Outro(StackableStateResult::Pop)
            }
            unknown => {
                return StackableStateResult::Error(anyhow!(
                    "Unknown menu return value: {unknown}"
                ));
            }
        }
        StackableStateResult::Continue
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        if let Err(e) = self.luamenu.step(timestep) {
            return StackableStateResult::Error(e.into());
        }

        match self.anim_state {
            AnimState::Normal => {}
            AnimState::Intro => self.intro_outro_anim -= timestep,
            AnimState::Outro(_) => self.intro_outro_anim += timestep,
        };

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
                // Note: starfield is static until the game starts
                self.starfield.borrow_mut().step(timestep);
                if self.intro_outro_anim > 1.0 {
                    self.intro_outro_anim = 1.0;
                    let ret = std::mem::replace(&mut self.anim_state, AnimState::Intro);
                    if let Err(e) = self.luamenu.reload() {
                        return StackableStateResult::Error(e.into());
                    }

                    return match ret {
                        AnimState::Outro(ret) => {
                            self.starfield = Rc::new(RefCell::new(Self::new_starfield(
                                self.renderer.borrow().size(),
                            )));
                            ret
                        }
                        _ => unreachable!(),
                    };
                }
            }
        }

        StackableStateResult::Continue
    }
}
