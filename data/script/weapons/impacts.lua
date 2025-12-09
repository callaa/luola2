-- This file contains code for bullet impact functions.
local tableutils = require("utils.table")
local Scheduler = require("utils.scheduler")
local Level = require("level")
local impacts = {}

-- generic function for explosions
-- count is the number of bullets generated
-- pos is the center of the explosion
-- template is the bullet template to which pos and vel properties are added
function impacts.make_shrapnell(count, pos, template)
	for a = 0, 360, (360 / count) do
		game.effect(
			"AddBullet",
			tableutils.combined(template, {
				pos = pos + Vec2_for_angle(a, 3.0),
				vel = Vec2_for_angle(a, 1000.0),
			})
		)
	end
end

-- Create firestarter bullets
-- these are short-lived projectiles that pass through ground and only interact with
-- combustible terrain to start fires
function impacts.make_firestarters(count, pos)
	impacts.make_shrapnell(count, pos, {
		terrain_collision = "passthrough",
		color = 0,
		texture = textures.get("dot3x3"),
		state = {
			scheduler = Scheduler.destroy_this,
			on_impact = impacts.firestarter,
		},
		timer = 0.3,
	})
end

function impacts.firestarter(this, terrain, obj)
	if Level.is_burnable(terrain) then
		this:destroy()
		game.effect("AddDynamicTerrain", {
			pos = this.pos,
			type = "Fire",
		})
	end
end

-- call an object's (ship, critter, etc.) bullet impact handler
-- If the impact handler returns true, the bullet's own impact
-- handling should not be called as the target's handler already
-- performed a special case action.
local function hit_object(bullet, obj, damage)
	if obj and obj.state and obj.state.on_bullet_hit then
		return obj.state.on_bullet_hit(obj, bullet, damage)
	end
end

-- Standard bullet
function impacts.bullet(this, terrain, obj)
	if hit_object(this, obj, 3) then
		return
	end

	this:destroy()
	game.effect("MakeBulletHole", this.pos)
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("boom"),
	})
end

-- Digger particle. Doesn't damage ships much
function impacts.diggerbeam(this, terrain, obj)
	if hit_object(this, obj, 0.3) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", {
		pos = this.pos,
		r = 2,
		dust = 0.5
	})
end

-- Special weapon grenade
function impacts.grenade(this, terrain, obj)
	if hit_object(this, obj, 1) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})
	impacts.make_shrapnell(36, this.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		state = {
			on_impact = impacts.bullet,
		},
	})
	impacts.make_firestarters(8, this.pos)
end

-- Special weapon Megabomb
function impacts.megabomb(this, terrain, obj)
	if hit_object(this, obj, 20) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 16 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	impacts.make_shrapnell(10, this.pos, {
		mass = 300,
		radius = 5,
		texture = textures.get("pewpew"),
		state = {
			on_impact = impacts.grenade,
		}
	})

	impacts.make_firestarters(8, this.pos)
end

-- Special weapon Rocket (should be slightly less powerful than a megabomb)
function impacts.rocket(this, terrain, obj)
	if hit_object(this, obj, 15) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 12 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	impacts.make_shrapnell(4, this.pos, {
		texture = textures.get("pewpew"),
		state = {
			on_impact = impacts.grenade,
		}
	})
	impacts.make_firestarters(8, this.pos)
end

-- Special weapon Homing Missile (should be less powerful than a rocket)
function impacts.missile(this, terrain, obj)
	if hit_object(this, obj, 10) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	impacts.make_shrapnell(20, this.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		state = {
			on_impact = impacts.bullet,
		}
	})
	impacts.make_firestarters(8, this.pos)
end

-- Mini missiles are small (possibly homing) missiles that are typically
-- launched in great numbers do don't do much damage on their own
function impacts.minimissile(this, terrain, obj)
	if hit_object(this, obj, 5) then
		return
	end

	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 5 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	impacts.make_firestarters(3, this.pos)
end

function impacts.foam_grenade(this, terrain, obj)
	if hit_object(this, obj, 0) then
		return
	end

	this:destroy()
	game.effect("AddDynamicTerrain", {
		pos = this.pos,
		type = "Foam",
	})
end

function impacts.greygoo(this, terrain, obj)
	if hit_object(this, obj, 0) then
		return
	end

	this:destroy()

	if obj and obj.state and obj.state.on_touch_greygoo then
		obj.state.on_touch_greygoo(obj)
	else
		game.effect("AddDynamicTerrain", {
			pos = this.pos,
			type = "GreyGoo",
		})
	end
end

function impacts.freezer(this, terrain, obj)
	if hit_object(this, obj, 0) then
		return
	end

	this:destroy()

	if obj and obj.is_ship then
		obj.frozen = true
		Scheduler.add_to_object(obj, 5, function(ship)
			ship.frozen = false
		end)
	else
		game.effect("AddDynamicTerrain", {
			pos = this.pos,
			type = "Freezer",
		})
	end
end

function impacts.nitroglycerin(this, terrain, obj)
	if hit_object(this, obj, 0) then
		return
	end

	this:destroy()
	-- Note: critters have special handling for nitro bullets
	game.effect("AddDynamicTerrain", {
		pos = this.pos,
		type = "Nitro",
	})
end

return impacts
