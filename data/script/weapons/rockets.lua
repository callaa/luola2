local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")
local maths = require("utils.maths")
local Rockets = {}

local function rocket_thrust(rocket)
	rocket:impulse(rocket.state.impulse)

	game.effect("AddParticle", {
		pos = rocket.pos,
		color = 0xffffffff,
		target_color = 0x00ff0000,
		lifetime = 0.15,
		texture = textures.get("dot3x3"),
	})

	return 0
end

function Rockets.rocket(pos, vel, angle, owner)
	game.effect("AddBullet", {
		pos = pos,
		vel = vel + Vec2_for_angle(angle, 100.0),
		mass = 300,
		radius = 5,
		owner = owner,
		texture = textures.get("rocket"),
		on_impact = Impacts.rocket,
		state = {
			impulse = Vec2_for_angle(angle, 8000.0),
			scheduler = Scheduler:new():add(0, rocket_thrust),
		},
		timer = 0,
	})
end

local function homing_missile_targeting(this)
	local target = nil
	local nearest = 0

	game.ships_iter(function(ship)
		local dist = ship.pos:dist(this.pos)
		if ship.player ~= this.owner and not ship.cloaked and (target == nil or dist < nearest) then
			target = ship.pos
			nearest = dist
		end
	end)

	if target ~= nil then
		local angle = -(target - this.pos):angle()
		local my_angle = -this.vel:angle()
		local boost = 10000
		if maths.angle_difference(my_angle, angle) > 60 then
			game.effect("AddParticle", {
				pos = this.pos,
				color = 0xffffffff,
				target_color = 0x00ff0000,
				lifetime = 0.15,
				texture = textures.get("dot8x8"),
			})
			boost = boost * 5
		end
		local impulse = Vec2_for_angle(angle, boost)
		this:impulse(impulse)

		game.effect("AddParticle", {
			pos = this.pos,
			color = 0xffffffff,
			target_color = 0x00ff0000,
			lifetime = 0.15,
			texture = textures.get("dot3x3"),
		})
	end

	return 0.02
end

function Rockets.homing_missile(pos, vel, launch_angle, owner)
	game.effect("AddBullet", {
		pos = pos,
		vel = vel + Vec2_for_angle(launch_angle, 100.0),
		mass = 300,
		radius = 5,
		owner = owner,
		texture = textures.get("rocket"),
		on_impact = Impacts.missile,
		state = {
			angle = launch_angle,
			scheduler = Scheduler:new():add(0, homing_missile_targeting),
		},
		timer = 0,
	})
end

function Rockets.mini_homing_missile(pos, vel, launch_angle, owner)
	game.effect("AddBullet", {
		pos = pos,
		vel = vel + Vec2_for_angle(launch_angle, 100.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = owner,
		texture = textures.get("rocket"),
		on_impact = Impacts.minimissile,
		state = {
			angle = launch_angle,
			scheduler = Scheduler:new():add(0, homing_missile_targeting),
		},
		timer = 0,
	})
end

return Rockets
