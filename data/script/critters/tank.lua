local Scheduler = require("utils.scheduler")
local Level = require("utils.level")
local Rockets = require("weapons.rockets")
local maths = require("utils.maths")

local Tank = {}

function Tank._on_touch_ledge(critter)
	critter.walking = -critter.walking
end

function Tank._on_touch_ground(critter, ter)
	if ter == 0x80 then
		critter:destroy()
	end
end

local FIRING_DIST2 = 800 * 800

function Tank._seek_target(critter)
	local nearest_enemy_pos = nil
	local nearest_enemy_dist2 = FIRING_DIST2

	game.ships_iter(function(ship)
		if ship.player ~= critter.owner then
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
		critter.texture = textures.get("tank_attack")

		Scheduler.add_to_object(critter, 0.045, function(critter)
			Tank._fire(critter, nearest_enemy_pos)
		end)

		return 0.5
	elseif critter.walking == 0 then
		critter.texture = textures.get("tank")
		critter.walking = critter.facing
	end

	return 1
end

function Tank._fire(critter, target_pos)
	local firing_angle = (target_pos - critter.pos):angle()
	Rockets.mini_homing_missile(critter.pos + Vec2(0, -8), Vec2(0, -10), -firing_angle, critter.owner)
end

function Tank._on_bullet_hit(critter, bullet)
	critter:destroy()
	game.effect("AddParticle", {
		pos = critter.pos,
		texture = textures.get("bigboom"),
	})
end

function Tank:new(pos)
	local tank = {
		scheduler = Scheduler:new():add(1, Tank._seek_target),
	}
	setmetatable(tank, self)
	self.__index = self
	return tank
end

function Tank.create(pos, owner)
	game.effect("AddCritter", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 100,
		radius = 6,
		walking = 1,
		texture = textures.get("tank"),
		state = Tank:new(pos),
		owner = owner,
		waterproof = false,
		on_bullet_hit = Tank._on_bullet_hit,
		on_touch_ledge = Tank._on_touch_ledge,
		on_touch_ground = Tank._on_touch_ground,
		timer = 0,
	})
end

return Tank
