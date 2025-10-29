-- This file contains code for bullet impact functions.
local tableutils = require("utils.table")

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

-- Standard bullet
function impacts.bullet(this, terrain, ship)
	this:destroy()
	game.effect("MakeBulletHole", this.pos)
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("boom"),
	})

	if ship ~= nil then
		ship:damage(3)
	end
end

-- Special weapon grenade
function impacts.grenade(this, terrain, ship)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if ship ~= nil then
		ship:damage(1)
	end

	impacts.make_shrapnell(36, this.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		on_impact = impacts.bullet,
	})
end

-- Special weapon Megabomb
function impacts.megabomb(this, terrain, ship)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 16 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if ship ~= nil then
		ship:damage(20)
	end

	impacts.make_shrapnell(10, this.pos, {
		mass = 300,
		radius = 5,
		texture = textures.get("pewpew"),
		on_impact = impacts.grenade,
	})
end

-- Special weapon Rocket (should be slightly less powerful than a megabomb)
function impacts.rocket(this, terrain, ship)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 12 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if ship ~= nil then
		ship:damage(15)
	end

	impacts.make_shrapnell(4, this.pos, {
		texture = textures.get("pewpew"),
		on_impact = impacts.grenade,
	})
end

-- Special weapon Homing Missile (should be less powerful than a rocket)
function impacts.missile(this, terrain, ship)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

	if ship ~= nil then
		ship:damage(10)
	end

	impacts.make_shrapnell(20, this.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		on_impact = impacts.bullet,
	})
end

return impacts
