local Impacts = require("weapons.impacts")
local Scheduler = require("utils.scheduler")

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

function Pilot.create(pos, player, controller)
	game.effect("AddPilot",
		{
			pos = pos,
			controller = controller,
			player = player,
			state = {
				on_shoot = on_shoot,
				on_jetpack = on_jetpack,
				on_bullet_hit = on_bullet_hit,
			},
			walk_texture = textures.get("pilot_walk"),
			swim_texture = textures.get("pilot_swim"),
			jetpack_texture = textures.get("pilot_jetpack"),
			stand_texture = textures.get("pilot_stand"),
			parachute_texture = textures.get("pilot_parachuting"),
		}
	)
end

return Pilot
