local Impacts = require("weapons.impacts")

local weapons = {}

function weapons.cannon(ship)
	ship.primary_weapon_cooldown = 0.15

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		color = 0xffffffc0,
		radius = 5,
		owner = ship.player,
		texture = textures.get("pewpew"),
		state = {
			on_impact = Impacts.bullet,
		},
	})
end

function weapons.delta_cannon(ship)
	-- Dual cannons means (almost) twice as fast firing rate
	-- The offset positions make digging harder as a tradeoff
	ship.primary_weapon_cooldown = 0.09

	local bullet = {
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		color = 0xffffffe0,
		radius = 5,
		owner = ship.player,
		texture = textures.get("pewpew"),
		state = {
			on_impact = Impacts.bullet,
		},
	}

	if ship.state.barrel_switch then
		bullet.pos = ship.pos + Vec2_for_angle(-ship.angle + 60, 16)
	else
		bullet.pos = ship.pos + Vec2_for_angle(-ship.angle - 60, 16)
	end
	game.effect("AddBullet", bullet)
	ship.state.barrel_switch = not ship.state.barrel_switch
end

return weapons
