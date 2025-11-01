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

use anyhow::Result;
use log::error;

use crate::{
    game::{
        PlayerId,
        hud::draw_hud,
        level::{LEVEL_SCALE, LevelInfo, Starfield},
        objects::{Critter, GameObjectArray},
    },
    gfx::{AnimatedTexture, RenderMode, RenderOptions, Renderer},
    math::{Rect, Vec2},
};

use super::{
    controller::GameController,
    level::{Level, LevelEditor},
    objects::{GameObject, Particle, Projectile, Ship},
    scripting::ScriptEnvironment,
};

#[derive(Clone, Debug)]
pub enum WorldEffect {
    AddShip(Ship),
    AddBullet(Projectile),
    AddMine(Projectile),
    AddParticle(Particle),
    MakeBulletHole(Vec2),
    MakeBigHole(Vec2, i32),
    AddCritter(Critter),
    EndRound(PlayerId),
}

impl mlua::UserData for WorldEffect {}
impl mlua::FromLua for WorldEffect {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "WorldEffect".to_owned(),
                message: Some("expected WorldEffect".to_string()),
            }),
        }
    }
}
/**
 * The game world and everything in it
 */
pub struct World {
    scripting: ScriptEnvironment,
    level: Rc<RefCell<Level>>,

    /// Double buffered ships so we can access them in lua at any time
    ships: Rc<RefCell<GameObjectArray<Ship>>>, // ships that (may) be piloted by a player
    ships_work: RefCell<GameObjectArray<Ship>>,

    /// Bullets are fast moving projectiles that do not collide with each other.
    /// No double-buffering because bullets cannot be referenced in scripts outside their callbacks
    bullets: GameObjectArray<Projectile>,

    /// Mines are (typically) slow moving projectiles that may be collide with each other
    mines: Rc<RefCell<GameObjectArray<Projectile>>>,
    mines_work: RefCell<GameObjectArray<Projectile>>,

    /// Critters are creatures that fly, swim, or walk around the game world.
    /// They can be hostile or neutral.
    critters: Rc<RefCell<GameObjectArray<Critter>>>,
    critters_work: Rc<RefCell<GameObjectArray<Critter>>>,

    // particles that interact with terrain only
    /// Decorative particles with no interactions with anything.
    /// No need to double buffer these, since they don't do anything except get drawn on screen.
    particles: GameObjectArray<Particle>, // decorative particles with no interactions at all

    /// Noise texture used for viewports of dead players
    noise_texture: AnimatedTexture,

    /// Starfield background
    starfield: Option<Starfield>,

    /// This will be set to the winner of the round when decided
    winner: Option<PlayerId>,

    /// Debug helper
    debug_mode: DebugMode,
}

enum DebugMode {
    None,
    DrawTileGrid,
    DrawTileContentHints,
}

impl World {
    pub fn new(levelinfo: &LevelInfo, renderer: Rc<RefCell<Renderer>>) -> Result<Self> {
        let level = Rc::new(RefCell::new(Level::load_level(
            &renderer.borrow(),
            levelinfo,
        )?));
        let mut scripting = ScriptEnvironment::new(renderer.clone())?;

        let ships = Rc::new(RefCell::new(GameObjectArray::new()));
        let mines = Rc::new(RefCell::new(GameObjectArray::new()));
        let critters = Rc::new(RefCell::new(GameObjectArray::new()));

        scripting.init_game(
            level.clone(),
            ships.clone(),
            mines.clone(),
            critters.clone(),
        )?;

        if let Some(levelscript) = levelinfo.script_path() {
            scripting.load_level_specific_script(&levelscript)?;
        }

        // TODO load level init script if specified

        Ok(World {
            scripting,
            level,
            ships,
            ships_work: RefCell::new(GameObjectArray::new()),
            bullets: GameObjectArray::new(),
            mines,
            mines_work: RefCell::new(GameObjectArray::new()),
            critters,
            critters_work: Rc::new(RefCell::new(GameObjectArray::new())),
            particles: GameObjectArray::new(),
            noise_texture: AnimatedTexture::new(
                renderer.borrow().texture_store().find_texture("noise")?,
            ),
            starfield: if levelinfo.use_starfield() {
                Some(Starfield::new())
            } else {
                None
            },
            winner: None,
            debug_mode: DebugMode::None,
        })
    }

    pub fn scripting(&self) -> &ScriptEnvironment {
        &self.scripting
    }

    pub fn on_screensize_change(&mut self, new_viewport_size: (i32, i32)) {
        if let Some(sf) = self.starfield.as_mut() {
            sf.recalculate(new_viewport_size.0 as f32, new_viewport_size.1 as f32);
        }
    }

    pub fn apply_accumulated_effects(&mut self) {
        let effects = self.scripting.take_accumulated_effects().into_iter();
        self.apply_effects(effects);
    }

    pub fn apply_effects<I>(&mut self, effects: I)
    where
        I: Iterator<Item = WorldEffect>,
    {
        let mut level = self.level.borrow_mut();
        let mut level_editor = LevelEditor::new(&mut level);
        for fx in effects {
            match fx {
                WorldEffect::AddShip(s) => self.ships.borrow_mut().push(s),
                WorldEffect::AddBullet(b) => self.bullets.push(b),
                WorldEffect::AddMine(b) => self.mines.borrow_mut().push(b),
                WorldEffect::AddParticle(p) => self.particles.push(p),
                WorldEffect::MakeBulletHole(pos) => {
                    level_editor.make_standard_bullet_hole(pos, &mut self.scripting)
                }
                WorldEffect::MakeBigHole(pos, r) => {
                    level_editor.make_hole(pos, r, &mut self.scripting)
                }
                WorldEffect::AddCritter(critter) => {
                    self.critters.borrow_mut().push(critter);
                }
                WorldEffect::EndRound(winner) => self.winner = Some(winner),
            }
        }
    }

    pub fn toggle_debugmode(&mut self) {
        self.debug_mode = match self.debug_mode {
            DebugMode::None => DebugMode::DrawTileGrid,
            DebugMode::DrawTileGrid => DebugMode::DrawTileContentHints,
            DebugMode::DrawTileContentHints => DebugMode::None,
        }
    }

    /**
     * Simulate a physics step
     *
     * Returns the ID of the winning player, if a win was decided in this step.
     */
    pub fn step(&mut self, controllers: &[GameController], timestep: f32) -> Option<PlayerId> {
        let level = self.level.borrow();
        let lua = self.scripting.lua();

        //
        // Simulation step phase
        //

        // Ship simulation step
        {
            let mut work = self.ships_work.borrow_mut();
            for ship in self.ships.borrow().iter() {
                work.push(ship.step(
                    if ship.controller() > 0 {
                        Some(&controllers[(ship.controller() - 1) as usize])
                    } else {
                        None
                    },
                    &level,
                    lua,
                    timestep,
                ));
            }

            work.sort();
        }

        // Bullet simulation step
        for bullet in self.bullets.iter_mut() {
            bullet.step_mut(&level, lua, timestep);
        }

        self.bullets.sort();

        // Mine simulation step
        {
            let mut work = self.mines_work.borrow_mut();
            for mine in self.mines.borrow().iter() {
                if !mine.is_destroyed() {
                    work.push(mine.step(&level, lua, timestep));
                }
            }
            work.sort();
        }

        // Critter simulation step
        {
            let mut work = self.critters_work.borrow_mut();
            for critter in self.critters.borrow().iter() {
                if !critter.is_destroyed() {
                    work.push(critter.step(&level, lua, timestep));
                }
            }
            work.sort();
        }

        // Decorative particle simulation step
        for p in self.particles.iter_mut() {
            p.step_mut(timestep);
        }

        self.particles.sort();

        drop(level);

        //
        // Collision check phase
        //

        // Ships can collide with other ships, bullets, and mines
        {
            let mut work = self.ships_work.borrow_mut();
            let mut minework = self.mines_work.borrow_mut();
            let mut critterwork = self.critters_work.borrow_mut();
            for (ship, rest) in work.self_collision_iter_mut() {
                // Ship self collisions
                for other in rest {
                    if let Some(impulse) = ship.physics().check_collision(other.physics()) {
                        ship.physics_mut().add_impulse(impulse);
                        other.physics_mut().add_impulse(impulse * -1.0);
                    }
                }

                // Ship to bullet checks.
                for bullet in self.bullets.collider_slice_mut(ship).iter_mut() {
                    if bullet.owner() != ship.player_id()
                        && let Some(impulse) = ship.physics().check_collision(bullet.physics())
                    {
                        ship.physics_mut().add_impulse(impulse);
                        bullet.impact(0, Some(ship), lua);
                    }
                }

                // Ship to mine checks
                for mine in minework.collider_slice_mut(ship).iter_mut() {
                    if mine.owner() != ship.player_id()
                        && let Some(impulse) = ship.physics().check_collision(mine.physics())
                    {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        ship.physics_mut().add_impulse(impulse);
                        mine.impact(terrain, Some(ship), lua);
                    }
                }

                // Ship to critter checks
                for critter in critterwork.collider_slice_mut(ship).iter_mut() {
                    if let Some(impulse) = ship.physics().check_collision(critter.physics()) {
                        ship.physics_mut().add_impulse(impulse);
                        critter.physics_mut().add_impulse(impulse * -1.0);
                    }
                }
            }
        }

        // Mines can collide with bullets and other mines
        {
            let mut work = self.mines_work.borrow_mut();
            // Mine self collisions
            for (mine, rest) in work.self_collision_iter_mut() {
                for other in rest {
                    if mine.physics().check_overlap(other.physics()) {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        mine.impact(terrain, None, self.scripting.lua());
                        other.impact(terrain, None, self.scripting.lua());
                    }
                }

                // Bullet collisions. No friendly fire here
                for bullet in self.bullets.collider_slice_mut(mine).iter_mut() {
                    if mine.physics().check_overlap(bullet.physics()) {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        bullet.impact(terrain, None, self.scripting.lua());
                        mine.impact(terrain, None, self.scripting.lua());
                    }
                }
            }
        }

        // Critters can be hit by bullets and mines (and each other)
        {
            let mut work = self.critters_work.borrow_mut();
            for (critter, rest) in work.self_collision_iter_mut() {
                for other in rest {
                    if let Some(impulse) = critter.physics().check_collision(other.physics()) {
                        critter.physics_mut().add_impulse(impulse);
                        other.physics_mut().add_impulse(impulse * -1.0);
                    }
                }

                for bullet in self.bullets.collider_slice_mut(critter).iter_mut() {
                    // Drones are liable to shoot each other much too easily, so
                    // friendly fire is not checked
                    if (critter.owner() == 0 || critter.owner() != bullet.owner())
                        && critter.physics().check_overlap(bullet.physics())
                    {
                        // Critters may have special processing for bullets
                        // If bullet_hit returns false, it means the critter's script
                        // has already performed the special impact routine for the
                        // bullet (or wants it ignored otherwise.)
                        if critter.bullet_hit(bullet, self.scripting.lua()) {
                            let terrain = self.level.borrow().terrain_at(bullet.pos());
                            bullet.impact(terrain, None, self.scripting.lua());
                        }
                    }
                }

                let mut minework = self.mines_work.borrow_mut();
                for mine in minework.collider_slice_mut(critter).iter_mut() {
                    if critter.physics().check_overlap(mine.physics()) {
                        if critter.bullet_hit(mine, self.scripting.lua()) {
                            let terrain = self.level.borrow().terrain_at(mine.pos());
                            mine.impact(terrain, None, self.scripting.lua());
                        }
                    }
                }
            }
        }

        // Rotate working sets
        self.ships.swap(&self.ships_work);
        self.ships_work.borrow_mut().clear();

        self.mines.swap(&self.mines_work);
        self.mines_work.borrow_mut().clear();

        self.critters.swap(&self.critters_work);
        self.critters_work.borrow_mut().clear();

        // Global timers
        self.noise_texture.step(timestep);
        self.scripting.step_global_timer(timestep);

        // Apply accumulated effects
        self.apply_accumulated_effects();

        self.winner
    }

    //fn find_player(&self, id: i32) -> Option<Ref<'_, Ship>> {
    fn find_player(&self, id: i32) -> Option<Ship> {
        for s in self.ships.borrow().iter() {
            if s.player_id() == id {
                return Some(s.clone());
                //return Some(Ref::map(self.ships.borrow(), |ships| &ships.get_at(idx)));
            }
        }
        None
    }

    /**
     * Render a viewport for a specific player
     */
    pub fn render(&self, renderer: &mut Renderer, player_id: i32, viewport: Rect) {
        if let Err(err) = renderer.set_viewport(viewport) {
            error!("Couldn't set viewport: {}", err);
        }

        if let Some(player) = self.find_player(player_id) {
            let level = self.level.borrow();
            let camera_rect =
                level.camera_rect(player.pos(), viewport.w() as f32, viewport.h() as f32);
            let camera_pos = camera_rect.topleft();

            // Level background artwork
            if let Some(sf) = self.starfield.as_ref() {
                sf.render(renderer);
            }

            level.render(renderer, camera_rect);

            match self.debug_mode {
                DebugMode::None => {}
                DebugMode::DrawTileGrid => {
                    renderer.draw_debug_grid(camera_rect.topleft(), 64.0 * LEVEL_SCALE)
                }
                DebugMode::DrawTileContentHints => {
                    level.debug_render_tilehints(renderer, camera_rect)
                }
            }
            // World objects
            let left = camera_rect.x();
            let right = camera_rect.right();

            for particle in self.particles.range_slice(left, right) {
                particle.render(renderer, camera_pos);
            }

            for mine in self.mines.borrow().range_slice(left, right) {
                mine.render(renderer, camera_pos);
            }

            for bullet in self.bullets.range_slice(left, right) {
                bullet.render(renderer, camera_pos);
            }

            for ship in self.ships.borrow().range_slice(left, right) {
                ship.render(renderer, camera_pos);
            }

            for critter in self.critters.borrow().range_slice(left, right) {
                critter.render(renderer, camera_pos);
            }

            // Player HUD
            draw_hud(renderer, &player);
        } else {
            self.noise_texture.render(
                renderer,
                &RenderOptions {
                    mode: RenderMode::Tiled(6.0),
                    ..Default::default()
                },
            );
        }
    }

    pub fn level_size(&self) -> (f32, f32) {
        let level = self.level.borrow();
        (level.width(), level.height())
    }
}
