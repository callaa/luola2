local Scheduler = require("utils.scheduler")
local UniqID = require("utils.uniqid")
local Forcefields = require("forcefields")
local Level = require("level")
local sounds = require("sounds")

local Grav = {}

function Grav.create_gravmine(pos)
	game.effect("AddFixedObject", {
		pos = pos,
		texture = textures.get("gravmine"),
		id = UniqID.new(),
		state = {
			scheduler = Scheduler:new()
				:add(1, Grav._activate_mine)
				:add(1, Grav._gravmine_beams)
				:add(1, Grav._play_mine_warble)
				:add(15, Scheduler.destroy_this),
			on_destroy = Grav.on_destroy,
			radius = 700,
		},
		timer = 1,
	})
end

function Grav.create_moving_gravmine(pos, angle)
	game.effect("AddFixedObject", {
		pos = pos,
		texture = textures.get("gravmine"),
		id = UniqID.new(),
		state = {
			scheduler = Scheduler:new()
				:add(0.3, Grav._move_mine)
				:add(0.3, Grav._gravmine_beams)
				:add(0.3, Grav._play_mine_warble)
				:add(20, Scheduler.destroy_this),
			on_destroy = Grav.on_destroy,
			angle = angle,
			ff_id = UniqID.new(),
			radius = 500,
		},
		timer = 1,
	})
end

local function _gravmine_beam_impact(hs)
	if Level.is_destructible(hs.terrain) then
		game.effect("MakeBigHole", {
			pos = hs.stop,
			r = 1,
			dust = 1
		})
	end
end

function Grav._gravmine_beams(obj)
	-- Random hitscan beams that destroy landscape
	-- This is mainly to create a visual effect where bits of terrain is pulled towards
	-- the gravity well.
	local v = Vec2_for_angle(math.random(0, 360), obj.state.radius)
	local state = { on_done = _gravmine_beam_impact }

	game.effect("AddHitscan", {
		start = obj.pos,
		stop = obj.pos + v,
		state = state,
	})
	game.effect("AddHitscan", {
		start = obj.pos,
		stop = obj.pos - v,
		state = state,
	})

	return 0.2
end

function Grav._activate_mine(obj)
	local r = obj.state.radius
	obj.state.forcefield = Forcefields.update({
		bounds = { obj.pos.x - r, obj.pos.y - r, r * 2, r * 2 },
		point = 70,
	})
end

function Grav._move_mine(obj)
	local pos = obj.pos + Vec2_for_angle(obj.state.angle, 5)
	local r = obj.state.radius
	obj.pos = pos
	obj.state.forcefield = Forcefields.update({
		id = obj.state.ff_id,
		bounds = { pos.x - r, pos.y - r, r * 2, r * 2 },
		point = 60,
	})

	return 1/60
end

function Grav._play_mine_warble(obj)
	sfx.explosion(sounds.high_warble(), obj.pos, 0.05)
	return 0.4
end

function Grav.on_destroy(obj)
	if obj.state.forcefield ~= nil then
		game.effect("RemoveForcefield", obj.state.forcefield)
	end
end

function Grav.activate_shield(ship)
	ship.state.forcefield = UniqID.new()
	Scheduler.add_to_object(ship, 0, Grav._update_shield)
	Scheduler.add_to_object(ship, 0.1, Grav._consume_shield_energy)
	Scheduler.add_to_object(ship, 0, Grav._play_shield_warble)
end

function Grav.deactivate_shield(ship)
	game.effect("RemoveForcefield", ship.state.forcefield)
	ship.state.forcefield = nil
end

function Grav._consume_shield_energy(ship)
	if ship.state.forcefield ~= nil then
		ship.ammo = ship.ammo - 1
		if ship.ammo > 0 then
			game.player_effect("rumble", ship.controller, {
				low = 0.01,
				duration = 0.1,
			})
			return 0.1
		else
			Grav.deactivate_shield(ship)
		end
	end
end

function Grav._play_shield_warble(ship)
	if ship.state.forcefield ~= nil then
		sfx.weapon(sounds.high_warble())
		return 0.4
	end
end

function Grav._update_shield(ship)
	if ship.state.forcefield == nil then
		return
	end
	if ship.is_wrecked then
		Grav.deactivate_shield(ship)
		return
	end
	Forcefields.update({
		id = ship.state.forcefield,
		bounds = { ship.pos.x - 100, ship.pos.y - 100, 200, 200 },
		point = -500,
	})
	game.effect("AddParticle", {
		texture = textures.get("shield"),
		pos = ship.pos,
		lifetime = 0.1,
		target_color = 0x00ffffff,
	})
	return 0
end

return Grav
