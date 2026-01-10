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
	ship.primary_weapon_cooldown = 0.10

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

	if ship.state.barrel_switch == 1 then
		bullet.pos = ship.pos + Vec2_for_angle(-ship.angle + 60, 16)
		ship.state.barrel_switch = 2
	elseif ship.state.barrel_switch == 2 then
		bullet.pos = ship.pos + Vec2_for_angle(-ship.angle - 60, 16)
		ship.state.barrel_switch = 0
	else
		bullet.pos = ship.pos
		ship.state.barrel_switch = 1
	end
	game.effect("AddBullet", bullet)
end

return weapons
