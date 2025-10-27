require "primary_weapons"

function ship_thrust_effect(ship, uw)
	if uw then
		for i = 0,5 do
			game.effect("AddParticle", {
				pos = ship.pos,
				vel = Vec2_for_angle(-ship.angle - 180 + math.random(-60, 60), 100),
				color = 0x66aaaaff,
				target_color = 0x00aaaaff,
				lifetime = 0.30,
				texture = textures.get("dot"),
			})
		end
	else
		game.effect("AddParticle", {
			pos = ship.pos,
			vel = Vec2_for_angle(-ship.angle - 180, 300) + ship.vel,
			color = 0xffffffff,
			target_color = 0x00ff0000,
			lifetime = 0.15,
			texture = textures.get("dot"),
		})
	end
end

function ship_on_base(ship, timestep)
	ship:damage(-5 * timestep)
	ship.ammo = ship.ammo + timestep / 10
end

function on_ship_destroyed(ship)
	-- We can't check this immediately on ship destruction
	-- because we need to check the state of all ships/players
	-- but changes to those haven't been committed yet.
	global_scheduler_add(1, check_round_end_condition)

	game.effect("MakeBigHole", { pos = ship.pos, r = 16 })
	game.effect("AddParticle", {
		pos = ship.pos,
		texture = textures.get("bigboom"),
	})

    make_shrapnell(36, ship.pos, {
        color = 0xffff6666,
        mass = 30,
        radius = 1,
        drag = 0.0025,
        texture = textures.get("pewpew"),
        on_impact = bullet_impact,
    })
end

function check_round_end_condition()
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

luola_ships = {
	vwing = {
		title = "V-Wing",
		description = "Your basic caveflyer",
		template = {
			texture = textures.get("vwing"),
			mass = 1000,
			drag = 0.04,
			radius = 16,
			thrust = 40,
            hitpoints = 100,
			on_fire_primary = primary_weapon_cannon,
			on_destroyed = on_ship_destroyed,
			on_base = ship_on_base,
			on_thrust = ship_thrust_effect,
		}
	}
}

