local Scheduler = require("utils.scheduler")
local weapons = require("primary_weapons")
local bullets = require("bullets")

local function ship_thrust_effect(ship, uw)
	if uw then
		for i = 0, 5 do
			game.effect("AddParticle", {
				pos = ship.pos,
				vel = Vec2_for_angle(-ship.angle - 180 + math.random(-60, 60), 100),
				color = 0x66aaaaff,
				target_color = 0x00aaaaff,
				lifetime = 0.30,
				texture = textures.get("dot3x3"),
			})
		end
	else
		game.effect("AddParticle", {
			pos = ship.pos,
			vel = Vec2_for_angle(-ship.angle - 180, 300) + ship.vel,
			color = 0xffffffff,
			target_color = 0x00ff0000,
			lifetime = 0.15,
			texture = textures.get("dot8x8"),
		})
	end
end

local function ship_on_base(ship, timestep)
	local hp = ship.health
	ship:damage(-5 * timestep)

	if ship.health > hp then
		local r = ship.radius
		game.effect("AddParticle", {
			pos = ship.pos + Vec2(math.random() * r * 2 - r, math.random() * r * 2 - r),
			vel = Vec2(math.random(-60, 60), -160),
			a = Vec2(0, 9.8*50),
			color = 0xffffaa00,
			target_color = 0x00660000,
			lifetime = 1,
		})
	end
	ship.ammo = ship.ammo + timestep / 10
end

local function check_round_end_condition()
	local last_player_standing = 0
	local count = 0

	game.ships_iter(function(ship)
		if ship.player ~= 0 then
			count = count + 1
			if last_player_standing == 0 then
				last_player_standing = ship.player
			else
				last_player_standing = 0
				return false
			end
		end
	end)

	if count == 0 or last_player_standing ~= 0 then
		game.effect("EndRound", last_player_standing)
	end
end

local function on_ship_destroyed(ship)
	-- We can't check this immediately on ship destruction
	-- because we need to check the state of all ships/players
	-- but changes to those haven't been committed yet.
	Scheduler.add_global(1, check_round_end_condition)

	game.effect("MakeBigHole", { pos = ship.pos, r = 16 })
	for i = 0, 2 do
		game.effect("AddParticle", {
			pos = ship.pos + Vec2(math.random(-30, 30), math.random(-30, 30)),
			texture = textures.get("bigboom"),
			reveal_in = i / 6,
		})
	end

	bullets.make_shrapnell(36, ship.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		on_impact = bullets.bullet,
	})
end

local ships = {
	vwing = {
		title = "V-Wing",
		template = {
			texture = textures.get("vwing"),
			mass = 1000,
			drag = 0.04,
			radius = 16,
			thrust = 40,
			turn_speed = 260,
			hitpoints = 100,
			on_fire_primary = weapons.cannon,
			on_destroyed = on_ship_destroyed,
			on_base = ship_on_base,
			on_thrust = ship_thrust_effect,
		},
	},
}

return ships
