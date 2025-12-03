-- This file contains code for bullet impact functions.
local tableutils = require("utils.table")
local Scheduler = require("utils.scheduler")

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
	local tex = textures.get("dot3x3")

	for a = 0, 360, (360 / count) do
		game.effect("AddBullet", {
			pos = pos + Vec2_for_angle(a, 3.0),
			vel = Vec2_for_angle(a, 1000.0),
			terrain_collision = "passthrough",
			color = 0,
			texture = tex,
			state = {
				scheduler = Scheduler:new():add(0.3, Scheduler.destroy_this),
				on_impact = impacts.firestarter,
			},
			timer = 0.3,
		})
	end
end

function impacts.firestarter(this, terrain, obj)
	if terrain == 0x42 or terrain == 0x43 then
		this:destroy()
		game.effect("AddDynamicTerrain", {
			pos = this.pos,
			type = "Fire",
		})
	end
end

-- Standard bullet
function impacts.bullet(this, terrain, obj)
	this:destroy()
	game.effect("MakeBulletHole", this.pos)
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("boom"),
	})

	if obj and obj.damage then
		obj:damage(3)
	end
end

-- Digger particle. Doesn't damage ships much
function impacts.diggerbeam(this, terrain, obj)
	this:destroy()
	game.effect("MakeBigHole", {
		pos = this.pos,
		r = 2,
		dust = 0.5
	})

	if obj and obj.damage then
		obj:damage(0.3)
	end
end

-- Special weapon grenade
function impacts.grenade(this, terrain, obj)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if obj and obj.damage then
		obj:damage(1)
	end

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
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 16 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if obj and obj.damage then
		obj:damage(20)
	end

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
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 12 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if obj and obj.damage then
		obj:damage(15)
	end

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
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if obj and obj.damage then
		obj:damage(10)
	end

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
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 5 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if obj and obj.damage then
		obj:damage(5)
	end
	impacts.make_firestarters(3, this.pos)
end

function impacts.foam_grenade(this, terrain, obj)
	this:destroy()
	game.effect("AddDynamicTerrain", {
		pos = this.pos,
		type = "Foam",
	})
end

function impacts.greygoo(this, terrain, obj)
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
	this:destroy()
	-- Note: critters have special handling for nitro bullets
	game.effect("AddDynamicTerrain", {
		pos = this.pos,
		type = "Nitro",
	})
end

return impacts
