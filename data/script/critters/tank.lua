local Scheduler = require("utils.scheduler")
local Level = require("level")
local Rockets = require("weapons.rockets")
local maths = require("utils.maths")
local UniqID = require("utils.uniqid")

local Tank = {}

function Tank.on_touch_ledge(critter)
	critter.walking = -critter.walking
end

function Tank.on_touch_ground(critter, ter)
	if ter == 0x80 then
		critter:destroy()
	end
end

local FIRING_DIST2 = 800 * 800

function Tank._seek_target(critter)
	local nearest_enemy_pos = nil
	local nearest_enemy_dist2 = FIRING_DIST2

	game.ships_iter(function(ship)
		if ship.player ~= critter.owner and not ship.cloaked then
			if ship.pos.y <= critter.pos.y then
				local angle = (ship.pos - critter.pos):angle()
				if angle > 35 and angle < 145 then
					local dist2 = ship.pos:dist_squared(critter.pos)
					if dist2 < nearest_enemy_dist2 then
						nearest_enemy_pos = ship.pos
						nearest_enemy_dist2 = dist2
					end
				end
			end
		end
	end)

	if nearest_enemy_pos ~= nil then
		critter.walking = 0
		critter.facing = maths.signum(nearest_enemy_pos.x - critter.pos.x)
		critter.action = true

		Scheduler.add_to_object(critter, 3/60, function(critter)
			Tank._fire(critter, nearest_enemy_pos)
		end)

		return 2
	elseif critter.walking == 0 then
		critter.walking = critter.facing
	end

	return 1
end

function Tank._fire(critter, target_pos)
	local firing_angle = (target_pos - critter.pos):angle()
	Rockets.mini_homing_missile(critter.pos + Vec2(0, -8), Vec2(0, -10), -firing_angle, critter.owner)
	local ammo = critter.state.ammo - 1
	if ammo < 0 then
		critter:destroy()
	else
		critter.state.ammo = ammo
	end
end

function Tank.on_bullet_hit(critter, bullet)
	critter:destroy()
end

function Tank.on_destroy(critter)
	game.effect("AddParticle", {
		pos = critter.pos,
		texture = textures.get("bigboom"),
	})
end

function Tank:new(pos)
	local tank = {
		scheduler = Scheduler:new():add(1, Tank._seek_target),
		is_tank = true,
		ammo = 5,
	}
	setmetatable(tank, self)
	self.__index = self
	return tank
end

function Tank.create(pos, owner)
	game.effect("AddCritter", {
		id = UniqID.new(),
		pos = pos,
		vel = Vec2(0, 0),
		mass = 100,
		radius = 6,
		walking = 1,
		texture = textures.get("tank"),
		action_texture = textures.get("tank_attack"),
		state = Tank:new(pos),
		owner = owner,
		waterproof = false,
		timer = 0,
	})
end

-- Count the number of tanks deployed by this player in the area
function Tank.count(player, pos)
	local count = 0
	game.critters_iter(pos, 1000, 0, function(c)
		if c.owner == player and c.state.is_tank then
			count = count + 1
		end
	end)
	return count
end

return Tank
