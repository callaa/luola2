local Impacts = require("weapons.impacts")
local Scheduler = require("utils.scheduler")
local Level = require("level")
local Portals = require("portals")

local Pilot = {}

local function on_shoot(pilot)
	pilot.weapon_cooldown = 0.4

	game.effect("AddBullet", {
		pos = pilot.pos + Vec2(0, -16),
		vel = pilot.vel + pilot:aim_vector(1000.0),
		color = 0xffff6666,
		radius = 5,
		owner = pilot.player,
		texture = textures.get("pewpew"),
		state = {
			on_impact = Impacts.bullet,
		},
	})
end

local function on_jetpack(pilot, dir)
	pilot:impulse(Vec2(dir * -2000, -4000))
	if math.random(0, 2) == 0 then
		local exhaust_angle = 90 - 45 * dir
		local exhaust_offset = Vec2(-8 * pilot.facing, -16)

		game.effect("AddParticle", {
			pos = pilot.pos + exhaust_offset,
			vel = Vec2_for_angle(exhaust_angle, 300) + pilot.vel,
			color = 0xffffffff,
			target_color = 0x00ff0000,
			lifetime = 0.15,
			texture = textures.get("dot8x8"),
		})
	end
end

local function on_ninjarope_swing(pilot, dir)
	local tangent = pilot.rope_tangent
	local exhaust_offset = Vec2(-8 * pilot.facing, -16)
	if dir < 0 then
		tangent = Vec2(-tangent.x, -tangent.y)
	end

	pilot:impulse(tangent * -2000)

	game.effect("AddParticle", {
		pos = pilot.pos + exhaust_offset,
		vel = tangent * 300 + pilot.vel,
		color = 0x80ffffff,
		target_color = 0x00ff0000,
		lifetime = 0.15,
		texture = textures.get("dot8x8"),
	})
end

local function on_bullet_hit(pilot, bullet, damage)
	if damage <= 0 then
		return
	end

	Scheduler.add_global(1, check_round_end_condition)

	pilot:destroy()
	-- blood splatter
	for a = 0, 360, (360 / 16) do
		game.effect("AddTerrainParticle", {
			pos = pilot.pos,
			vel = Vec2_for_angle(a + math.random(-30, 30), 300.0),
			color = 0x80ff0000,
			imass = 1,
			drag = 0.002,
			stain = true,
		})
	end
end

local function on_ship_recall(pilot, terrain)
	-- First see if the ship still exists somewhere in the level
	local found = nil
	game.ships_iter(function(ship)
		if ship.player == pilot.player and ship.controller == 0 then
			found = ship.pos
			return false
		end
	end)

	if Level.is_base(terrain) then
		-- If the pilot is standing on a base, they can teleport
		-- their ship back or summon a replacement
		local pos = pilot.pos + Vec2(0, -60)
		if found then
			Portals.create_portal_pair(found, pos)
		else
			Portals.create_exit_portal(pos)
			create_ship_for_player(pilot.player, pos, false)
		end
	end
end

local function on_touch_ship(pilot, ship)
	-- Claim unoccupied ship
	if ship.controller == 0 then
		ship.controller = pilot.controller
		ship.player = pilot.player
		pilot:destroy()
	end
end

function Pilot.create(pos, player, controller)
	game.effect("AddPilot",
		{
			pos = pos,
			controller = controller,
			player = player,
			state = {
				on_shoot = on_shoot,
				on_jetpack = on_jetpack,
				on_ninjarope_swing = on_ninjarope_swing,
				on_bullet_hit = on_bullet_hit,
				on_ship_recall = on_ship_recall,
				scheduler = Scheduler:new():add(2, function(pilot)
					-- We don't want to immediately get back into a ship we just exited
					pilot.state.on_touch_ship = on_touch_ship
				end),
			},
			timer = 2,
			walk_texture = textures.get("pilot_walk"),
			swim_texture = textures.get("pilot_swim"),
			jetpack_texture = textures.get("pilot_jetpack"),
			stand_texture = textures.get("pilot_stand"),
			parachute_texture = textures.get("pilot_parachuting"),
		}
	)
end

return Pilot
