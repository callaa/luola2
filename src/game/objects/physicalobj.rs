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

use crate::game::{level::Level, level::terrain};
use crate::math::{LineF, Vec2};
use either::Either;

pub const SCALE_FACTOR: f32 = 50.0;

#[derive(Clone, Debug)]
pub enum TerrainCollisionMode {
    Exact,       // check every pixel on the line from old to new position
    Simple,      // just check the new pixel, may clip through thin terrain strips
    Passthrough, // pass through the terrain but return the terrain type
    None,        // terrain collisions disabled (except for level boundaries)
}

impl TerrainCollisionMode {
    pub fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalObject {
    pub pos: Vec2,
    pub vel: Vec2,
    pub imass: f32,    // 1.0 / mass
    pub radius: f32,   // used for collision detection
    pub drag: f32,     // drag coefficient used for resistance and buyouancy
    pub impulse: Vec2, // impulse accumulator
    pub terrain_collision_mode: TerrainCollisionMode,
}

impl PhysicalObject {
    /**
     * Perform a physics simulation step.
     *
     */
    pub fn step(&mut self, level: &Level, timestep: f32) -> (terrain::Terrain, terrain::Terrain) {
        let old_terrain = level.terrain_at(self.pos);
        let is_water = terrain::is_water(old_terrain);

        let density = if is_water { 60.0 } else { 1.2 };

        let g = 9.81 * SCALE_FACTOR;

        // Add impulse
        self.vel = self.vel + self.impulse * self.imass;
        self.impulse = Vec2::ZERO;

        let vv = self.vel.dot(self.vel);

        let mut a =
            // Gravity
            Vec2(0.0, g)
            // Air/water resistance (capped so we don't bounce when entering water)
            - (self.vel.normalized() * (0.5 * density * vv * 0.1 * self.drag).min(vv * timestep))
            // Buoyancy
            - Vec2(0.0, g * (density * self.drag))
            ;

        // Force fields
        for ff in &level.forcefields {
            if ff.bounds.contains(self.pos) {
                a = a + ff.uniform_force * SCALE_FACTOR;

                if ff.point_force != 0.0 {
                    let point = ff.bounds.center();
                    let delta = point - self.pos;
                    let dist = delta.dot(delta).sqrt();
                    if dist > 3.0 {
                        let normal = delta / dist;

                        // Make the gravity field less realistic and more fun.
                        let fudged_dist = 2.0 * dist / ff.bounds.w() + 1.0;
                        let force = ff.point_force / fudged_dist.powf(1.5);
                        a = a + normal * (force * SCALE_FACTOR);
                    }
                }
            }
        }

        // Euler integration works well enough here
        self.vel = self.vel + a * timestep;

        let new_pos = self.pos + self.vel * timestep;

        // Object already embedded in ground?
        if terrain::is_solid(old_terrain) && !self.terrain_collision_mode.is_none() {
            self.vel = Vec2::ZERO;
            (old_terrain, old_terrain)
        } else {
            // Terrain collision detection treats the object as a point particle
            // because that's good enough for this game.
            (
                old_terrain,
                match self.terrain_collision_mode {
                    TerrainCollisionMode::Exact => {
                        match level.terrain_line(LineF(self.pos, new_pos)) {
                            Either::Left((t, exact_pos)) => {
                                self.vel = Vec2::ZERO;
                                self.pos = exact_pos;
                                t
                            }
                            Either::Right(t) => {
                                self.pos = new_pos;
                                t
                            }
                        }
                    }
                    TerrainCollisionMode::Simple => {
                        let t = level.terrain_at(new_pos);
                        if terrain::is_solid(t) {
                            if terrain::is_ice(t) {
                                // TODO we could check the terrain slope at new_pos
                                // and have the ship slide uphill depending on
                                // its vector, but this simple horizontal sliding
                                // works pretty well even on its own.
                                let newvel = Vec2(self.vel.0, 0.0);
                                let new_pos = self.pos + newvel * timestep;
                                if !terrain::is_solid(level.terrain_at(new_pos)) {
                                    self.vel = newvel;
                                    self.pos = new_pos;
                                } else {
                                    self.vel = Vec2::ZERO;
                                }
                            } else {
                                // Regular non-slippery terrain
                                self.vel = Vec2::ZERO;
                            }
                        } else {
                            self.pos = new_pos;
                        }
                        t
                    }
                    TerrainCollisionMode::Passthrough => {
                        let t = level.terrain_at(new_pos);
                        if terrain::is_level_boundary(t) {
                            self.vel = Vec2::ZERO;
                        } else {
                            self.pos = new_pos;
                        }
                        t
                    }
                    TerrainCollisionMode::None => {
                        let t = level.terrain_at(new_pos);
                        if terrain::is_level_boundary(t) {
                            self.vel = Vec2::ZERO;
                            t
                        } else {
                            self.pos = new_pos;
                            0
                        }
                    }
                },
            )
        }
    }

    pub fn add_impulse(&mut self, impulse: Vec2) {
        self.impulse = self.impulse + impulse
    }

    /**
     * Check if this object collides with the other object.
     *
     * If a collision is detected, the impulse that should be added to this
     * object is returned. Add the negative of the impulse to the other object
     * to balance the forces.
     */
    pub fn check_collision(&self, other: &PhysicalObject) -> Option<Vec2> {
        let distv = self.pos - other.pos;
        let dd = distv.dot(distv);
        let r = self.radius + other.radius;

        if dd > r * r {
            return None;
        }

        // We have overlap. Calculate collision normal vector
        let normal = distv.normalized();

        // Combined velocity
        let collv = self.vel - other.vel;
        let impact_speed = collv.dot(normal);

        // Are the objects already moving away from each other?
        if impact_speed > 0.0 {
            return Some(Vec2::ZERO);
        }

        // Coefficient of restitution
        let cor = 0.95;

        // Collision impulse
        let j = -(1.0 + cor) * impact_speed / (self.imass + other.imass);

        Some(normal * j)
    }

    /**
     * Collision check without impulse calculation
     */
    pub fn check_overlap(&self, other: &PhysicalObject) -> bool {
        let distv = self.pos - other.pos;
        let dd = distv.dot(distv);
        let r = self.radius + other.radius;

        dd <= r * r
    }
}
