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

pub type Terrain = u8;

pub(super) const TER_BIT_WATER: Terrain = 0b10000000;
pub(super) const TER_BIT_DESTRUCTIBLE: Terrain = 0b01000000;
pub(super) const TER_BIT_DYNAMIC: Terrain = 0b00100000;
pub const TER_MASK_SOLID: Terrain = 0b00011111;
pub(super) const TER_TYPE_GROUND: Terrain = 1;
pub(super) const TER_TYPE_BURNABLE: Terrain = 2;
pub(super) const TER_TYPE_CINDER: Terrain = 3;
pub(super) const TER_TYPE_EXPLOSIVE: Terrain = 4;
pub(super) const TER_TYPE_HIGH_EXPLOSIVE: Terrain = 5;
pub(super) const TER_TYPE_ICE: Terrain = 6;
pub(super) const TER_TYPE_BASE: Terrain = 7;
pub(super) const TER_TYPE_NOREGENBASE: Terrain = 8; // base that does not regenerate
pub(super) const TER_TYPE_BASESUPPORT: Terrain = 9; // ground that regenerates if base regeneration is enabled
pub(super) const TER_TYPE_WALKWAY: Terrain = 10; // ground that walkers can walk through
pub(super) const TER_TYPE_GREYGOO: Terrain = 11; // can infect a ship with grey goo
pub(super) const TER_TYPE_DAMAGE: Terrain = 12; // damages things that touch it

pub(super) const TER_LEVELBOUND: Terrain = TER_MASK_SOLID; // special type indicating level boundary

/// Is this terrain point open space (i.e. not solid and not water)
pub fn is_space(t: Terrain) -> bool {
    t & (TER_BIT_WATER | TER_MASK_SOLID) == 0
}

/// Can a walking critter walk through this type of terrain
pub fn can_walk_through(t: Terrain) -> bool {
    is_space(t) || (t & TER_MASK_SOLID) == TER_TYPE_WALKWAY
}

/// Is this terrain point open water?
pub fn is_water(t: Terrain) -> bool {
    t & (TER_BIT_WATER | TER_MASK_SOLID) == TER_BIT_WATER
}

/// Is this terrain underwater? (solid or free space)
pub fn is_underwater(t: Terrain) -> bool {
    t & TER_BIT_WATER == TER_BIT_WATER
}

/// Is this dynamic terrain?
/// Solid ground is animated as falling sand. Dynamic terrain is not initially active,
/// but shooting at it will trigger an activation wave with "LooseningSand" cells.
/// After remaining still for a while, sand deactivates again.
/// Dynamic water is not implemented at the moment.
pub fn is_dynamic(t: Terrain) -> bool {
    t & TER_BIT_DYNAMIC == TER_BIT_DYNAMIC
}

/// Is this terrain point something you can't fly through?
pub fn is_solid(t: Terrain) -> bool {
    t & TER_MASK_SOLID != 0
}

/// Is this solid terrain that cannot be erased?
pub fn is_indestructible_solid(t: Terrain) -> bool {
    is_solid(t) && !is_destructible(t)
}

/// Is this (normal) explosive terrain?
pub fn is_explosive(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_EXPLOSIVE
}

/// Is this (highly) explosive terrain?
pub fn is_high_explosive(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_HIGH_EXPLOSIVE
}

/// Is this burnable terrain
pub fn is_burnable(t: Terrain) -> bool {
    t & TER_TYPE_CINDER >= TER_TYPE_BURNABLE
}

/// Is this burnable terrain that turns into cinder
pub fn is_cinder(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_CINDER
}

/// Is this ice/snow?
/// Ships will slide on icy surfaces
pub fn is_ice(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_ICE
}

/// Is this a base?
/// Note: bases orient the ship to point upwards. Underwater bases point the ship downward
pub fn is_base(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_BASE
}

/// Is this a non-regenerating base?
/// Works otherwise the same as a regular base but does not regenerate even
/// when regeneration is enabled.
pub fn is_noregen_base(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_NOREGENBASE
}

/// Is this active base material of any type?
pub fn is_effective_base(t: Terrain) -> bool {
    is_base(t) || is_noregen_base(t)
}

/// Is this base support terrain?
/// This is essentially regular ground, but regenerates along with bases
/// if base regeneration is enabled.
pub fn is_basesupport(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_BASESUPPORT
}

/// Is this grey goo?
/// Grey goo is created by the grey goo dynamic terrain effect or may exist in the level as a permanent hazard.
/// Touching it can infect a ship.
pub fn is_greygoo(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_GREYGOO
}

/// Does this terrain inflict damage
pub fn is_damaging(t: Terrain) -> bool {
    t & TER_MASK_SOLID == TER_TYPE_DAMAGE
}

/**
 * Is this terrain you can blow up?
 * If a non-solid terrain pixel is marked as destructible,
 * its artwork pixel can be cleared even if the pixel was free space to begin with.
 */
pub fn is_destructible(t: Terrain) -> bool {
    t & TER_BIT_DESTRUCTIBLE == TER_BIT_DESTRUCTIBLE
}

/// Level boundary reached?
pub fn is_level_boundary(t: Terrain) -> bool {
    t == TER_LEVELBOUND
}
