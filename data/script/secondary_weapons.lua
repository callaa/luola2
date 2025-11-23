local Impacts = require("weapons.impacts")
local Scheduler = require("utils.scheduler")
local Drone = require("critters.drone")
local Tank = require("critters.tank")
local Mines = require("weapons.mines")
local Rockets = require("weapons.rockets")
local Grav = require("weapons.grav")
local Hitscan = require("weapons.hitscan")

local weapons = {}

function weapons.grenade(ship)
	if ship:consume_ammo(5, 0.4) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("pewpew"),
			state = {
				on_impact = Impacts.grenade,
			}
		})
	end
end

function weapons.megabomb(ship)
	if ship:consume_ammo(10, 1.0) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = Vec2(ship.vel.x, math.max(0, ship.vel.y)),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("megabomb"),
			state = {
				on_impact = Impacts.megabomb,
			},
		})
	end
end

function weapons.rocket(ship)
	if ship:consume_ammo(10, 1.0) then
		Rockets.rocket(ship.pos, ship.vel, -ship.angle, ship.player)
	end
end

function weapons.missile(ship)
	if ship:consume_ammo(8, 1.0) then
		Rockets.homing_missile(ship.pos, ship.vel, -ship.angle, ship.player)
	end
end

function weapons.mine(ship)
	if ship:consume_ammo(10, 0.4) then
		Mines.create_mine(ship.pos, ship.player)
	end
end

function weapons.magmine(ship)
	if ship:consume_ammo(10, 0.4) then
		Mines.create_magmine(ship.pos, ship.player)
	end
end

function weapons.landmine(ship)
	if Mines.detonate_landmine(ship.player) then
		ship.secondary_weapon_cooldown = 0.2
		return
	end

	if ship:consume_ammo(10, 0.2) then
		Mines.create_landmine(ship.pos, ship.angle, ship.player)
	end
end

function weapons.drone(ship)
	ship.secondary_weapon_cooldown = 0.4
	local ammo = ship.ammo - 20
	if ammo >= 0 then
		if Drone.count(ship.player, ship.pos) < 3 then
			ship.ammo = ammo
			Drone.create(ship.pos, ship.player)
		else
			game.player_effect("hud_overlay", ship.player, {
				text = textures.font("menu", "Cannot deploy more drones here!"),
				pos = Vec2(0.5, 0.1),
				color = 0xffff0000,
				lifetime = 2,
				fadeout = 1,
			})
		end
	end
end

function weapons.tank(ship)
	ship.secondary_weapon_cooldown = 0.4
	local ammo = ship.ammo - 20
	if ammo >= 0 then
		if Tank.count(ship.player, ship.pos) < 3 then
			ship.ammo = ammo
			Tank.create(ship.pos, ship.player)
		else
			game.player_effect("hud_overlay", ship.player, {
				text = textures.font("menu", "Cannot deploy more tanks here!"),
				pos = Vec2(0.5, 0.1),
				color = 0xffff0000,
				lifetime = 2,
				fadeout = 1,
			})
		end
	end
end

function weapons.cloaking_device(ship)
	ship.secondary_weapon_cooldown = 0.6
	if ship.cloaked then
		ship.cloaked = false
	elseif ship.ammo >= 0.5 then
		ship.cloaked = true
		Scheduler.add_to_object(ship, 0.1, function(ship)
			if ship.cloaked then
				local ammo = ship.ammo - 0.5
				if ammo < 0 then
					ship.cloaked = false
					return
				end
				ship.ammo = ammo
				return 0.1
			end
		end)

		-- Cool special effect
		for i = 0, 360, 36 do
			game.effect("AddParticle", {
				pos = ship.pos,
				vel = ship.vel + Vec2_for_angle(i, 100),
				angle = ship.angle,
				texture = ship.texture,
				lifetime = 0.5,
				color = game.player_color(ship.player) - 0x80000000,
				target_color = game.player_color(ship.player) - 0xff000000,
			})
		end
	end
end

function weapons.ghostship(ship)
	ship.secondary_weapon_cooldown = 0.6
	if ship.ghostmode then
		ship.ghostmode = false
	elseif ship.ammo > 0.9 then
		ship.ghostmode = true
		Scheduler.add_to_object(ship, 0.1, function(ship)
			if ship.ghostmode then
				local ammo = ship.ammo - 0.9
				if ammo < 0 then
					ship.ghostmode = false
					return
				end
				ship.ammo = ammo
				return 0.1
			end
		end)
	end
end

function weapons.gravmine(ship)
	if ship:consume_ammo(33, 2.0) then
		Grav.create_gravmine(ship.pos)
	end
end

function weapons.shield(ship)
	ship.secondary_weapon_cooldown = 0.2
	if ship.state.forcefield ~= nil then
		Grav.deactivate_shield(ship)
	elseif ship.ammo > 1 then
		Grav.activate_shield(ship)
	end
end

function weapons.foam_grenade(ship)
	if ship:consume_ammo(10, 0.4) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("dot8x8"),
			color = 0xffbc990f,
			state = {
				on_impact = Impacts.foam_grenade,
			}
		})
	end
end

function weapons.greygoo(ship)
	if ship:consume_ammo(10, 0.4) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("dot8x8"),
			color = 0xffcccccc,
			state = {
				on_impact = Impacts.greygoo,
			}
		})
	end
end

function weapons.freezer(ship)
	if ship:consume_ammo(10, 0.4) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("dot8x8"),
			color = 0xffb7f5fc,
			state = {
				on_impact = Impacts.freezer,
			}
		})
	end
end

function weapons.nitroglycerin(ship)
	if ship:consume_ammo(10, 0.4) then
		game.effect("AddBullet", {
			pos = ship.pos,
			vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
			mass = 300,
			radius = 5,
			drag = 0.0025,
			owner = ship.player,
			texture = textures.get("dot8x8"),
			color = 0xfffc2292,
			state = {
				is_nitro = true,
				on_impact = Impacts.nitroglycerin,
			},
		})
	end
end

function weapons.laser(ship)
	if ship:consume_ammo(0.8, 0.2) then
		-- note: hitscan is performed on the next frame
		Hitscan.laser(ship.pos + Vec2_for_angle(-ship.angle, 16) + ship.vel / 60, ship.angle, ship.player)
	end
end

return weapons
