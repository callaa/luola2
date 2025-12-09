local Level = require("level")
local Hitscan = require("weapons.hitscan")
local Impacts = require("weapons.impacts")
local Maths = require("utils.maths")
local Scheduler = require("utils.scheduler")

local Turrets = {}

local function deathray_turret_target(turret)
	local target = game.ships_nearest_pos(turret.pos, turret.state.range, 0)
	if target then
		local angle = (target - turret.pos):angle()
		Hitscan.deathray(
			turret.pos + Vec2_for_angle(-angle, 8),
			-- Misaim just a little bit so it's not a guaranteed hit on a fast moving ship
			angle + math.random(-5, 5),
			0
		)
		turret:action()
	end
	return 1.5
end

local function turret_hit_bullet(turret, bullet)
	if bullet.state and bullet.state.is_toxin then
		-- toxins do not affect mechanical turrets
		return true
	end

	turret:destroy()
	game.effect("AddParticle", {
		pos = turret.pos,
		texture = textures.get("bigboom"),
	})
	game.effect("MakeBigHole", { pos = turret.pos, r = 8 })
end

local function gun_turret_target(turret)
	local target = game.ships_nearest_pos(turret.pos, turret.state.range, 0)
	local new_angle
	local fire_at_will = false

	if target then
		local target_angle = (target - turret.pos):angle()
		local ad = Maths.angle_difference(target_angle, turret.angle)
		local abs_ad = math.abs(ad)
		new_angle = (turret.angle + Maths.signum(ad) * math.min(abs_ad, 30)) % 360
		fire_at_will = abs_ad < 16
	else
		new_angle = (turret.angle + turret.state.turn_dir) % 360
	end

	local barrel_pos = turret.pos + Vec2_for_angle(-new_angle, 16)
	if Level.mask_solid(game.terrain_at(barrel_pos)) ~= 0 then
		turret.state.turn_dir = -turret.state.turn_dir
	else
		turret.angle = new_angle
	end

	if fire_at_will then
		game.effect("AddBullet", {
			pos = barrel_pos,
			vel = Vec2_for_angle(-turret.angle, 1000.0),
			color = 0xffff6666,
			radius = 5,
			owner = 0,
			texture = textures.get("pewpew"),
			state = {
				on_impact = Impacts.bullet,
			},
		})
		return 0.2
	end		
	return 0.1
end

function Turrets.add_deathray(pos, range)
	game.effect("AddFixedObject", {
		pos = pos,
		id = 0,
		texture = textures.get("turret_deathray"),
		action_texture = textures.get("turret_deathray_shoot"),
		radius = 8,
		state = {
			range = range,
			scheduler = deathray_turret_target,
			on_bullet_hit = turret_hit_bullet,
		},
		timer = math.random() * 2, -- randomize scheduler start so all turrets don't fire at once
	})
end

function Turrets.add_gun(pos, range, initial_angle)
	game.effect("AddFixedObject", {
		pos = pos,
		id = 0,
		texture = textures.get("turret_gun"),
		radius = 8,
		angle = initial_angle,
		state = {
			turn_dir = 15,
			range = range,
			scheduler = gun_turret_target,
			on_bullet_hit = turret_hit_bullet,
		},
		timer = math.random() * 2, -- randomize scheduler start so all turrets don't fire at once
	})
end

function Turrets.add_from_config(turrets)
	for _, t in ipairs(turrets) do
		local pos = Level.to_world_coordinates(t.pos)
		if t.type == "deathray" then
			Turrets.add_deathray(pos, t.range)
		elseif t.type == "gun" then
			Turrets.add_gun(pos, t.range, t.angle)
		else
			print("Unknown turret type", t.type)
		end
	end
end

return Turrets