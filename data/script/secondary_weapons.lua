local bullets = require("bullets")
local Scheduler = require("utils.scheduler")
local trig = require("utils.trig")
local Drone = require("critters.drone")
local Mines = require("weapons.mines")

local weapons = {}

function weapons.grenade(ship)
	local ammo = ship.ammo - 0.05
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("pewpew"),
		on_impact = bullets.grenade,
	})
end

function weapons.megabomb(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 1.0

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel,
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("megabomb"),
		on_impact = bullets.megabomb,
	})
end

function weapons.rocket(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 1.0

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 100.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("rocket"),
		on_impact = bullets.rocket,
		state = {
			impulse = Vec2_for_angle(-ship.angle, 8000.0),
			scheduler = Scheduler:new():add(0, function(p)
				p:impulse(p.state.impulse)

				game.effect("AddParticle", {
					pos = p.pos,
					color = 0xffffffff,
					target_color = 0x00ff0000,
					lifetime = 0.15,
					texture = textures.get("dot3x3"),
				})

				return 0
			end),
		},
		timer = 0,
	})
end

function weapons.missile(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 1.0

	game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 100.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("rocket"),
		on_impact = bullets.missile,
		state = {
			angle = -ship.angle,
			scheduler = Scheduler:new():add(0, function(this)
				local target = nil
				local nearest = 0

				game.ships_iter(function(ship)
					local dist = ship.pos:dist(this.pos)
					if ship.player ~= this.owner and (target == nil or dist < nearest) then
						target = ship.pos
						nearest = dist
					end
				end)

				if target ~= nil then
					local target_angle = -(target - this.pos):angle()
					local turn = trig.angle_difference(this.state.angle, target_angle)
					if turn < 0 then
						this.state.angle = this.state.angle - 20
					else
						this.state.angle = this.state.angle + 20
					end
					local impulse = Vec2_for_angle(this.state.angle, 10000)
					this:impulse(impulse)

					game.effect("AddParticle", {
						pos = this.pos,
						color = 0xffffffff,
						target_color = 0x00ff0000,
						lifetime = 0.15,
						texture = textures.get("dot3x3"),
					})
				end

				return 0.02
			end),
		},
		timer = 0,
	})
end

function weapons.mine(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	Mines.create_mine(ship.pos, ship.player)
end

function weapons.magmine(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	Mines.create_magmine(ship.pos, ship.player)
end

function weapons.landmine(ship)
	if Mines.detonate_landmine(ship.player) then
		ship.secondary_weapon_cooldown = 0.1
		return
	end

	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	Mines.create_landmine(ship.pos, ship.angle, ship.player)
end

function weapons.drone(ship)
	local ammo = ship.ammo - 0.2
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	Drone.create(ship.pos, ship.player)
end

function weapons.cloaking_device(ship)
	ship.secondary_weapon_cooldown = 0.6
	if ship.cloaked then
		ship.cloaked = false
	elseif ship.ammo > 0 then
		ship.cloaked = true
		Scheduler.add_to_object(ship, 0.1, function(ship)
			if ship.cloaked then
				local ammo = ship.ammo - 0.005
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
	elseif ship.ammo > 0 then
		ship.ghostmode = true
		Scheduler.add_to_object(ship, 0.1, function(ship)
			if ship.ghostmode then
				local ammo = ship.ammo - 0.009
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

return weapons
