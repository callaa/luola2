local Impacts = require("weapons.impacts")

local weapons = {}

function weapons.cannon(ship)
	ship.primary_weapon_cooldown = 0.15

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		color = 0xffff6666,
		radius = 5,
		owner = ship.player,
		texture = textures.get("pewpew"),
		on_impact = Impacts.bullet,
	})
end

return weapons
