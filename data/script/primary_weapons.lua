require "bullets"

function primary_weapon_cannon(ship)
    ship.primary_weapon_cooldown = 0.15

    game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		color = 0xffff6666,
		mass = 30,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("pewpew"),
		on_impact=bullet_impact,
	})
end
