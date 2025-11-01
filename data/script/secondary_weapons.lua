local bullets = require("bullets")
local Scheduler = require("utils.scheduler")
local trig = require("utils.trig")
local Drone = require("critters.drone")

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
		texture = textures.get("megabomb"),
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
		texture = textures.get("megabomb"),
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
						texture = textures.get("dot"),
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

	local drag = 1 / 1.2
	if game.terrain_at(ship.pos) == 0x80 then
		drag = 1 / 60.0
	end

	game.effect("AddMine", {
		pos = ship.pos,
		vel = Vec2(0, 0),
		mass = 300,
		radius = 3,
		drag = drag,
		owner = ship.player,
		texture = textures.get("mine"),
		on_impact = bullets.grenade,
		state = {
			scheduler = Scheduler:new():add(1, function(this)
				this.texture = textures.get("mine_armed")
				this:disown()
			end),
		},
		timer = 1,
	})
end

function weapons.magmine(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then
		return
	end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	local drag = 1 / 1.2
	if game.terrain_at(ship.pos) == 0x80 then
		drag = 1 / 60.0
	end

	game.effect("AddMine", {
		pos = ship.pos,
		vel = Vec2(0, 0),
		mass = 300,
		radius = 8,
		drag = drag,
		owner = ship.player,
		texture = textures.get("magmine"),
		on_impact = bullets.grenade,
		state = {
			scheduler = Scheduler:new()
				:add(1, function(this)
					this.texture = textures.get("magmine_armed")
					this:disown()
				end)
				:add(0.5, function(this)
					local nearest_enemy_pos = nil
					local nearest_enemy_dist2 = 300 * 300

					game.ships_iter(function(ship)
						if ship.player ~= this.owner then
							local dist2 = ship.pos:dist_squared(this.pos)
							if dist2 < nearest_enemy_dist2 then
								nearest_enemy_pos = ship.pos
								nearest_enemy_dist2 = dist2
							end
						end
					end)

					if nearest_enemy_pos ~= nil then
						local a = (nearest_enemy_pos - this.pos):normalized() * (50000 / math.sqrt(nearest_enemy_dist2))
						this.vel = this.vel + a
						return 0.1
					end

					return 0.6
				end),
		},
		timer = 1,
	})
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

return weapons
