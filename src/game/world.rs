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
use smallvec::SmallVec;

use crate::{
    game::{
        Player, PlayerId, PlayerState,
        hud::{PlayerHud, draw_hud, draw_minimap},
        level::{DynamicTerrainCell, LEVEL_SCALE, LevelInfo, Starfield, terrain::Terrain},
        objects::{Critter, FixedObject, GameObjectArray, HitscanProjectile, TerrainParticle},
    },
    gfx::{AnimatedTexture, Color, RenderMode, RenderOptions, Renderer},
    math::{Rect, Vec2},
};

use super::{
    controller::GameController,
    level::{Forcefield, Level, LevelEditor},
    objects::{GameObject, Particle, Projectile, Ship},
    scripting::ScriptEnvironment,
};

#[derive(Clone, Debug)]
pub enum WorldEffect {
    AddShip(Ship),
    AddBullet(Projectile),
    AddMine(Projectile),
    AddParticle(Particle),
    AddTerrainParticle(TerrainParticle),
    AddDynamicTerrain(Vec2, DynamicTerrainCell),
    AddFixedObject(FixedObject),
    AddHitscan(HitscanProjectile),
    AddPixel(Vec2, Terrain, Color),
    ColorPixel(Vec2, Color),
    MakeBulletHole(Vec2),
    MakeBigHole {
        pos: Vec2,
        radius: i32,
        dust_chance: Option<f32>,
    },
    AddCritter(Critter),
    UpdateForcefield(Forcefield),
    RemoveForcefield(i32),
    SetWindspeed(f32),
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

    players: Rc<RefCell<Vec<PlayerState>>>,

    /// Double buffered ships so we can access them in lua at any time
    ships: Rc<RefCell<GameObjectArray<Ship>>>, // ships that (may) be piloted by a player
    ships_work: RefCell<GameObjectArray<Ship>>,

    /// Bullets are fast moving projectiles that do not collide with each other.
    /// No double-buffering because bullets cannot be referenced in scripts outside their callbacks
    bullets: GameObjectArray<Projectile>,

    /// Mines are (typically) slow moving projectiles that may be collide with each other
    /// No double buffering because mines do not need to reference each other in their callbacks
    mines: Rc<RefCell<GameObjectArray<Projectile>>>,

    /// Hitscan projectiles. These live for only one tick
    hitscans: Vec<HitscanProjectile>,

    /// Critters are creatures that fly, swim, or walk around the game world.
    /// They can be hostile or neutral.
    critters: Rc<RefCell<GameObjectArray<Critter>>>,
    critters_work: Rc<RefCell<GameObjectArray<Critter>>>,

    /// Terrain particle (e.g. snow and dust)
    terrainparticles: GameObjectArray<TerrainParticle>,

    /// Decorative particles with no interactions with anything.
    /// No need to double buffer these, since they don't do anything except get drawn on screen.
    particles: GameObjectArray<Particle>, // decorative particles with no interactions at all

    /// Game objects that are fixed in place (or move via script actions only)
    fixedobjects: Rc<RefCell<GameObjectArray<FixedObject>>>,

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
    pub fn new(
        players: &[Player],
        levelinfo: &LevelInfo,
        renderer: Rc<RefCell<Renderer>>,
    ) -> Result<Self> {
        let level = Rc::new(RefCell::new(Level::load_level(
            &renderer.borrow(),
            levelinfo,
        )?));
        let mut scripting = ScriptEnvironment::new(renderer.clone())?;

        let players = Rc::new(RefCell::new(
            players.iter().map(|_| PlayerState::new()).collect(),
        ));
        let ships = Rc::new(RefCell::new(GameObjectArray::new()));
        let mines = Rc::new(RefCell::new(GameObjectArray::new()));
        let critters = Rc::new(RefCell::new(GameObjectArray::new()));
        let fixedobjects = Rc::new(RefCell::new(GameObjectArray::new()));

        scripting.init_game(
            players.clone(),
            level.clone(),
            ships.clone(),
            mines.clone(),
            critters.clone(),
        )?;

        if let Some(levelscript) = levelinfo.script_path() {
            scripting.load_level_specific_script(&levelscript)?;
        }

        Ok(World {
            players,
            scripting,
            level,
            ships,
            ships_work: RefCell::new(GameObjectArray::new()),
            bullets: GameObjectArray::new(),
            mines,
            hitscans: Vec::new(),
            critters,
            critters_work: Rc::new(RefCell::new(GameObjectArray::new())),
            terrainparticles: GameObjectArray::new(),
            particles: GameObjectArray::new(),
            fixedobjects,
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
                WorldEffect::AddShip(s) => {
                    if s.player_id() > 0 && s.controller() > 0 {
                        self.players.borrow_mut()[s.player_id() as usize - 1].camera_pos = s.pos();
                    }
                    self.ships.borrow_mut().push(s);
                }
                WorldEffect::AddBullet(b) => self.bullets.push(b),
                WorldEffect::AddMine(b) => self.mines.borrow_mut().push(b),
                WorldEffect::AddParticle(p) => self.particles.push(p),
                WorldEffect::AddTerrainParticle(p) => self.terrainparticles.push(p),
                WorldEffect::AddDynamicTerrain(pos, t) => level_editor.add_dynterrain(pos, t),
                WorldEffect::AddFixedObject(o) => {
                    let mut objs = self.fixedobjects.borrow_mut();
                    objs.push(o);
                    objs.sort();
                }
                WorldEffect::AddHitscan(hs) => self.hitscans.push(hs),
                WorldEffect::ColorPixel(pos, color) => {
                    level_editor.color_point(pos, color);
                }
                WorldEffect::AddPixel(pos, ter, color) => {
                    level_editor.add_point(pos, ter, color);
                }
                WorldEffect::MakeBulletHole(pos) => {
                    level_editor.make_standard_bullet_hole(pos, &mut self.scripting)
                }
                WorldEffect::MakeBigHole {
                    pos,
                    radius,
                    dust_chance,
                } => level_editor.make_hole(
                    pos,
                    radius,
                    dust_chance.unwrap_or(0.05),
                    &mut self.scripting,
                ),
                WorldEffect::AddCritter(critter) => {
                    self.critters.borrow_mut().push(critter);
                }
                WorldEffect::UpdateForcefield(ff) => level_editor.update_forcefield(&ff),
                WorldEffect::RemoveForcefield(id) => level_editor.remove_forcefield(id),
                WorldEffect::SetWindspeed(ws) => level_editor.set_windspeed(ws),
                WorldEffect::EndRound(winner) => self.winner = Some(winner),
            }
        }
        level_editor.step_dynterrain();
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
        // Player state reset
        for ps in self.players.borrow_mut().iter_mut() {
            if ps.fadeout < 1.0 {
                ps.fadeout += timestep;
            }
            // this will get replaced with the right HUD type if the player is still in the game
            ps.hud = PlayerHud::None;

            // Overlay animations
            ps.overlays.retain_mut(|o| o.age(timestep));
        }

        let level = self.level.borrow();

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
                    self.scripting.lua(),
                    timestep,
                ));

                if ship.player_id() > 0 && ship.controller() > 0 {
                    let ps = &mut self.players.borrow_mut()[ship.player_id() as usize - 1];
                    let ship = work.last_mut();
                    ps.hud = PlayerHud::Ship {
                        health: ship.health(),
                        ammo: ship.ammo(),
                        cooling_down: ship.secondary_weapon_cooldown() > 0.0,
                        pos: ship.pos().element_wise_product(level.size_scale()),
                    };
                    // camera inertia for an enhanced feeling of motion
                    // TODO rather than trailing behind the ship, the camera should look ahead?
                    ps.camera_pos = ps.camera_pos + (ship.pos() - ps.camera_pos) / 5.0;
                    ps.fadeout = -1.0;
                }
            }

            work.sort();
        }

        // Bullet simulation step
        for bullet in self.bullets.iter_mut() {
            bullet.step_mut(&level, self.scripting.lua(), timestep);
        }

        self.bullets.sort();

        // Mine simulation step
        {
            let mut mines = self.mines.borrow_mut();
            for mine in mines.iter_mut() {
                mine.step_mut(&level, self.scripting.lua(), timestep);
            }
            mines.sort();
        }

        // Critter simulation step
        {
            let mut work = self.critters_work.borrow_mut();
            for critter in self.critters.borrow().iter() {
                if !critter.is_destroyed() {
                    work.push(critter.step(&level, self.scripting.lua(), timestep));
                }
            }
            work.sort();
        }

        // Terrain particle simulation step
        for tp in self.terrainparticles.iter_mut() {
            let e = tp.step_mut(&level, timestep);
            if let Some(e) = e {
                if tp.is_staining() {
                    self.scripting.add_effect(WorldEffect::ColorPixel(e.0, e.2));
                } else {
                    self.scripting
                        .add_effect(WorldEffect::AddPixel(e.0, e.1, e.2));
                }
            }
        }
        self.terrainparticles.sort();

        // Decorative particle simulation step
        let windspeed = level.windspeed();
        for p in self.particles.iter_mut() {
            p.step_mut(timestep, windspeed);
        }

        self.particles.sort();

        // Fixed object simulation step
        {
            let mut any_moved = false;
            let mut objects = self.fixedobjects.borrow_mut();
            for o in objects.iter_mut() {
                any_moved |= o.step_mut(self.scripting.lua(), timestep);
            }
            if any_moved {
                objects.sort();
            }
        }

        //
        // Collision check phase
        //

        // Ships can collide with other ships, bullets, and mines
        {
            let mut work = self.ships_work.borrow_mut();
            let mut minework = self.mines.borrow_mut();
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
                for bullet in self.bullets.collider_slice_mut(ship) {
                    if bullet.owner() != ship.player_id()
                        && let Some(impulse) = ship.physics().check_collision(bullet.physics())
                    {
                        ship.physics_mut().add_impulse(impulse);
                        bullet.impact(0, Some(ship), self.scripting.lua());
                    }
                }

                // Ship to mine checks
                for mine in minework.collider_slice_mut(ship) {
                    if mine.owner() != ship.player_id()
                        && let Some(impulse) = ship.physics().check_collision(mine.physics())
                    {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        ship.physics_mut().add_impulse(impulse);
                        mine.impact(terrain, Some(ship), self.scripting.lua());
                    }
                }

                // Ship to critter checks
                for critter in critterwork.collider_slice_mut(ship) {
                    if let Some(impulse) = ship.physics().check_collision(critter.physics()) {
                        ship.physics_mut().add_impulse(impulse);
                        critter.physics_mut().add_impulse(impulse * -1.0);
                    }
                }

                // Ship to terrain particles check. This is mainly so ship's don't get buried in snow
                for tp in self.terrainparticles.collider_slice_mut(ship) {
                    if let Some(impulse) = ship.physics().check_collision(tp.physics()) {
                        ship.physics_mut().add_impulse(impulse);
                        tp.physics_mut().add_impulse(impulse * -1.0);
                    }
                }
            }
        }

        // Mines can collide with bullets and other mines
        {
            let mut work = self.mines.borrow_mut();
            // Mine self collisions
            for (mine, rest) in work.self_collision_iter_mut() {
                for other in rest {
                    if mine.physics().check_overlap(other.physics()) {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        mine.impact(terrain, Some(other), self.scripting.lua());
                        other.impact(terrain, Some(mine), self.scripting.lua());
                    }
                }

                // Bullet collisions. No friendly fire here
                for bullet in self.bullets.collider_slice_mut(mine).iter_mut() {
                    if mine.physics().check_overlap(bullet.physics()) {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        bullet.impact(terrain, Some(mine), self.scripting.lua());
                        mine.impact(terrain, Some(bullet), self.scripting.lua());
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
                            bullet.impact(terrain, Some(critter), self.scripting.lua());
                        }
                    }
                }

                let mut minework = self.mines.borrow_mut();
                for mine in minework.collider_slice_mut(critter).iter_mut() {
                    if critter.physics().check_overlap(mine.physics())
                        && critter.bullet_hit(mine, self.scripting.lua())
                    {
                        let terrain = self.level.borrow().terrain_at(mine.pos());
                        mine.impact(terrain, Some(critter), self.scripting.lua());
                    }
                }
            }
        }

        // Hitscans
        for hs in self.hitscans.iter_mut() {
            hs.hit_level(&level);

            let left = hs.left();
            let right = hs.right();

            enum Nearest<'a> {
                None,
                Ship(&'a mut Ship),
                Mine(&'a mut Projectile),
                Critter(&'a mut Critter),
            }
            let mut nearest_object = Nearest::None;

            let mut ships_work = self.ships_work.borrow_mut();
            for ship in ships_work.range_slice_mut(left, right) {
                if (hs.owner() != ship.player_id()) && hs.do_hit_object(self.scripting.lua(), ship)
                {
                    nearest_object = Nearest::Ship(ship);
                }
            }

            let mut mines_work = self.mines.borrow_mut();
            for mine in mines_work.range_slice_mut(left, right) {
                if (hs.owner() == 0 || hs.owner() != mine.owner())
                    && hs.do_hit_object(self.scripting.lua(), mine)
                {
                    nearest_object = Nearest::Mine(mine);
                }
            }

            let mut critters_work = self.critters_work.borrow_mut();
            for critter in critters_work.range_slice_mut(left, right) {
                if (hs.owner() == 0 || hs.owner() != critter.owner())
                    && hs.do_hit_object(self.scripting.lua(), critter)
                {
                    nearest_object = Nearest::Critter(critter);
                }
            }

            match nearest_object {
                Nearest::None => {}
                Nearest::Ship(s) => {
                    hs.on_hit_object(self.scripting.lua(), s);
                }
                Nearest::Mine(m) => {
                    hs.on_hit_object(self.scripting.lua(), m);
                }
                Nearest::Critter(c) => {
                    hs.on_hit_object(self.scripting.lua(), c);
                }
            }

            hs.on_done(self.scripting.lua());
        }

        drop(level);

        // Rotate working sets
        self.ships.swap(&self.ships_work);
        self.ships_work.borrow_mut().clear();

        self.critters.swap(&self.critters_work);
        self.critters_work.borrow_mut().clear();

        self.hitscans.clear();

        // Global timers
        self.noise_texture.step(timestep);
        self.scripting.step_global_timer(timestep);

        // Apply accumulated effects
        self.apply_accumulated_effects();

        // Continuous garbage collection to avoid big pauses
        if let Err(err) = self.scripting.lua().gc_step() {
            log::error!("GC error: {}", err);
        }

        self.winner
    }

    /**
     * Render a viewport for a specific player
     */
    pub fn render(&self, renderer: &mut Renderer, player_id: i32, viewport: Rect) {
        if let Err(err) = renderer.set_viewport(viewport) {
            error!("Couldn't set viewport: {}", err);
        }

        let player = &self.players.borrow()[player_id as usize - 1];

        if player.fadeout < 1.0 {
            let level = self.level.borrow();
            let camera_rect =
                level.camera_rect(player.camera_pos, viewport.w() as f32, viewport.h() as f32);
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

            for o in self.fixedobjects.borrow().range_slice(left, right) {
                o.render(renderer, camera_pos);
            }

            for particle in self.particles.range_slice(left, right) {
                particle.render(renderer, camera_pos);
            }

            for tp in self.terrainparticles.range_slice(left, right) {
                tp.render(renderer, camera_pos);
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
            draw_hud(renderer, player.hud, &player.overlays);

            if let Some(minimap) = self.level.borrow().minimap() {
                let mut markers = SmallVec::<[(Color, Vec2); 6]>::new();
                let levelscale = self.level.borrow().size_scale();
                for ship in self.ships.borrow().iter() {
                    if ship.controller() > 0 && !ship.is_cloaked() {
                        markers.push((
                            Color::player_color(ship.player_id()),
                            ship.pos().element_wise_product(levelscale),
                        ));
                    }
                }

                draw_minimap(renderer, minimap, &markers);
            }
        }

        if player.fadeout > 0.0 {
            self.noise_texture.render(
                renderer,
                &RenderOptions {
                    mode: RenderMode::Tiled(6.0),
                    color: Color::new_rgba(1.0, 1.0, 1.0, player.fadeout.min(1.0)),
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
