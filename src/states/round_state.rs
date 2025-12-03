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

use mlua::LuaSerdeExt;
use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    game::{
        GameControllerSet, MenuButton, Player, PlayerId,
        level::{LevelInfo, to_level_scale},
        world::World,
    },
    gfx::{Color, RenderOptions, Renderer, TextureId},
    math::{Rect, RectF, Vec2},
    states::{
        StackableState, StackableStateResult,
        pause_state::{PauseReturn, PauseState},
    },
};

pub struct GameRoundState {
    renderer: Rc<RefCell<Renderer>>,
    controllers: Rc<RefCell<GameControllerSet>>,

    /// List of players in this game
    players: Vec<Player>,

    /// The game-world of the current round (if round is underway)
    world: World,

    /// Extra blank viewport to fill in when there's an uneven number of players
    filler_viewport: Option<RectF>,

    /// Game logo to draw in the filler viewport
    filler_logo: TextureId,
    filler_logo_rect: RectF,
    filler_logo_vel: Vec2,
}

/// Return round winner (0 for draw) and whether to quit the game early
pub struct RoundWinner(pub PlayerId, pub bool);

impl GameRoundState {
    pub fn new(
        players: Vec<Player>,
        level: &LevelInfo,
        controllers: Rc<RefCell<GameControllerSet>>,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let world = World::new(&players, level, renderer.clone())?;
        let lua = world.scripting().lua();

        // Call game init script
        let player_settings = lua.create_table()?;
        for (idx, p) in players.iter().enumerate() {
            let player = lua.create_table()?;
            player.set("player", idx + 1)?;
            player.set("controller", p.controller)?;
            player.set("ship", p.ship.clone())?;
            player.set("weapon", p.weapon.clone())?;
            player.set("spawn", p.spawn.map(to_level_scale))?;
            player.set("pilot_spawn", p.pilot_spawn.map(to_level_scale))?;
            player_settings.push(player)?;
        }

        let settings = lua.create_table()?;
        settings.set("players", player_settings)?;
        settings.set("level", lua.to_value(level.script_settings())?)?;

        world
            .scripting()
            .get_function("luola_init_game")?
            .call::<()>(settings)?;

        let filler_logo = renderer.borrow().texture_store().find_texture("gamelogo")?;

        let mut game = Self {
            renderer,
            controllers,
            players,
            world,
            filler_viewport: None,
            filler_logo,
            filler_logo_rect: RectF::new(0.0, 0.0, 1.0, 1.0),
            filler_logo_vel: Vec2(5.0 + fastrand::f32() * 10.0, 5.0 + fastrand::f32() * 10.0),
        };

        game.resize_screen();

        Ok(game)
    }
}

impl StackableState for GameRoundState {
    fn handle_menu_button(&mut self, button: MenuButton) -> StackableStateResult {
        match button {
            MenuButton::Back => {
                let pause_state = Box::new(match PauseState::new(self.renderer.clone()) {
                    Ok(s) => s,
                    Err(err) => return StackableStateResult::Error(err),
                });
                return StackableStateResult::Push(pause_state);
            }
            MenuButton::Debug => self.world.toggle_debugmode(),
            _ => {}
        }
        StackableStateResult::Continue
    }

    fn receive_return(&mut self, retval: Box<dyn std::any::Any>) -> StackableStateResult {
        if let Some(pauseret) = retval.downcast_ref::<PauseReturn>() {
            match pauseret {
                PauseReturn::Resume => StackableStateResult::Continue,
                PauseReturn::EndRound => {
                    StackableStateResult::Return(Box::new(RoundWinner(0, false)))
                }
                PauseReturn::EndGame => {
                    StackableStateResult::Return(Box::new(RoundWinner(0, true)))
                }
            }
        } else {
            StackableStateResult::Error(anyhow!(
                "Unhandled game state return type: {:?}",
                retval.type_id()
            ))
        }
    }

    fn resize_screen(&mut self) {
        let renderer = self.renderer.borrow();

        let level_size = self.world.level_size();
        let (viewports, filler) = assign_viewports(
            Rect::new(0, 0, renderer.width(), renderer.height()),
            level_size.0 as i32,
            level_size.1 as i32,
            self.players.len(),
        );

        for (viewport, player) in viewports.into_iter().zip(self.players.iter_mut()) {
            player.viewport = viewport;
        }
        self.filler_viewport = filler;

        if let Some(f) = filler {
            let fillertex = renderer.texture_store().get_texture(self.filler_logo);
            if fillertex.width() > f.w() || fillertex.height() > f.h() {
                let scale = (f.w() / fillertex.width()).min(f.h() / fillertex.height());
                let w = fillertex.width() * scale;
                let h = fillertex.height() * scale;

                self.filler_logo_rect =
                    RectF::new(f.x() + (f.w() - w) / 2.0, f.y() + (f.h() - h) / 2.0, w, h);
            }
        }

        self.world
            .on_screensize_change(self.players[0].viewport.size());
    }

    fn state_iterate(&mut self, timestep: f32) -> StackableStateResult {
        let winner = self.world.step(&self.controllers.borrow().states, timestep);

        if let Some(winner) = winner {
            return StackableStateResult::Return(Box::new(RoundWinner(winner, false)));
        }

        let mut renderer = self.renderer.borrow_mut();
        renderer.clear();

        for (idx, p) in self.players.iter().enumerate() {
            self.world.render(&mut renderer, idx as i32 + 1, p.viewport);
        }

        if let Err(err) = renderer.reset_viewport() {
            return StackableStateResult::Error(err.into());
        }

        if let Some(viewport) = self.filler_viewport {
            // Filler viewport DVD screensaver animation
            let mut newpos = self.filler_logo_rect.topleft() + self.filler_logo_vel * timestep;
            if newpos.0 < viewport.x() {
                newpos.0 = viewport.x();
                self.filler_logo_vel.0 *= -1.0;
            }

            if newpos.1 < viewport.y() {
                newpos.1 = viewport.y();
                self.filler_logo_vel.1 *= -1.0;
            }

            if newpos.0 + self.filler_logo_rect.w() > viewport.right() {
                newpos.0 = viewport.right() - self.filler_logo_rect.w();
                self.filler_logo_vel.0 *= -1.0;
            }

            if newpos.1 + self.filler_logo_rect.h() > viewport.bottom() {
                newpos.1 = viewport.bottom() - self.filler_logo_rect.h();
                self.filler_logo_vel.1 *= -1.0;
            }

            self.filler_logo_rect = RectF::new(
                newpos.0,
                newpos.1,
                self.filler_logo_rect.w(),
                self.filler_logo_rect.h(),
            );

            renderer.draw_filled_rectangle(viewport, &Color::new(0.1, 0.1, 0.15));
            renderer
                .texture_store()
                .get_texture(self.filler_logo)
                .render(
                    &renderer,
                    &RenderOptions {
                        dest: crate::gfx::RenderDest::Rect(self.filler_logo_rect),
                        ..Default::default()
                    },
                );
        }

        renderer.present();
        StackableStateResult::Continue
    }
}

fn assign_viewports(
    screen: Rect,
    max_width: i32,
    max_height: i32,
    player_count: usize,
) -> (Vec<Rect>, Option<RectF>) {
    if player_count < 2 {
        (
            vec![Rect::new(
                screen.x(),
                screen.y(),
                screen.w().min(max_width),
                screen.h().min(max_height),
            )],
            None,
        )
    } else if player_count < 3 {
        let w2 = screen.w() / 2;
        let hmax = screen.h().min(max_height);
        (
            vec![
                Rect::new(screen.x(), screen.y(), w2.min(max_width), hmax),
                Rect::new(screen.x() + w2, screen.y(), w2.min(max_width), hmax),
            ],
            None,
        )
    } else {
        let mut viewports = Vec::with_capacity(player_count);
        let w = screen.w() * 2 / (player_count + player_count % 2) as i32;
        let h = screen.h() / 2;
        let bottom = player_count / 2;
        let top = player_count - bottom;
        for i in 0..top {
            viewports.push(Rect::new(
                screen.x() + w * i as i32,
                screen.y(),
                w.min(max_width),
                h.min(max_height),
            ));
        }
        for i in 0..bottom {
            viewports.push(Rect::new(
                screen.x() + w * i as i32,
                screen.y() + h,
                w.min(max_width),
                h.min(max_height),
            ));
        }

        let filler = if player_count % 2 == 1 {
            Some(RectF::new(
                (screen.x() + w * bottom as i32) as f32,
                (screen.y() + h) as f32,
                w as f32,
                h as f32,
            ))
        } else {
            None
        };

        (viewports, filler)
    }
}
