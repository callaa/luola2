local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")
local UniqID = require("utils.uniqid")

local Drone = {}

local PURSUE_DIST2 = 600 * 600
local FIRING_DIST2 = 400 * 400

function Drone._timer_fly(critter)
	local delta = critter.state.target - critter.pos

	critter.vel = critter.vel + delta

	return 0.2
end

function Drone._timer_targeting(critter)
	local nearest_enemy_pos = nil
	local nearest_enemy_dist2 = PURSUE_DIST2

	game.ships_iter(function(ship)
		if ship.player ~= critter.owner and not ship.cloaked then
			local dist2 = ship.pos:dist_squared(critter.pos)
			if dist2 < nearest_enemy_dist2 then
				nearest_enemy_pos = ship.pos
				nearest_enemy_dist2 = dist2
			end
		end
	end)

	if nearest_enemy_pos ~= nil then
		-- Pursue nearby enemies and shoot if they're close enough
		critter.state.target = nearest_enemy_pos

		if nearest_enemy_dist2 < FIRING_DIST2 then
			critter.state.ammo = 3
			Scheduler.add_to_object(critter, 0, Drone._timer_shoot)
		end

		return 1
	else
		-- No enemy in sight, just move randomly
		for _ = 0, 8 do
			local new_target = critter.pos + Vec2_for_angle(math.random(0, 360), 100)
			local _, _, hit = game.terrain_line(critter.pos, new_target)
			if not hit then
				critter.state.target = new_target
				break
			end
		end
		return 2
	end
end

function Drone._timer_shoot(critter)
	local nearest_enemy_pos = nil
	local nearest_enemy_dist2 = FIRING_DIST2 -- firing distance
	game.ships_iter(function(ship)
		if ship.player ~= critter.owner then
			local dist2 = ship.pos:dist_squared(critter.pos)
			if dist2 < nearest_enemy_dist2 then
				nearest_enemy_pos = ship.pos
				nearest_enemy_dist2 = dist2
			end
		end
	end)

	if nearest_enemy_pos ~= nil then
		local firing_vector = (nearest_enemy_pos - critter.pos):normalized()
		game.effect("AddBullet", {
			pos = critter.pos + firing_vector * 10,
			vel = firing_vector * 1000.0,
			color = 0xffff6666,
			radius = 5,
			owner = critter.owner,
			texture = textures.get("pewpew"),
			state = {
				on_impact = Impacts.bullet,
			},
		})
	else
		return nil
	end

	local ammo = critter.state.ammo - 1
	if ammo > 0 then
		critter.state.ammo = ammo
		return 0.1
	else
		critter.state.magazines = critter.state.magazines - 1
		if critter.state.magazines < 0 then
			-- out of bullets. Drop out of the sky
			critter.drag = 0.001
			critter.state.scheduler = Scheduler:new()
		end
	end
end

function Drone.on_bullet_hit(critter, bullet)
	critter:destroy()
end

function Drone.on_touch_ground(critter, ter)
	if critter.state.magazines <= 0 or ter == 0x80 then
		critter:destroy()
	end
end

function Drone.on_destroy(critter)
	game.effect("AddParticle", {
		pos = critter.pos,
		texture = textures.get("bigboom"),
	})
end

function Drone:new(pos)
	local drone = {
		is_drone = true,
		target = pos,
		ammo = 3,
		magazines = 10,
		scheduler = Scheduler:new():add(0, Drone._timer_fly):add(1, Drone._timer_targeting),
	}
	setmetatable(drone, self)
	self.__index = self
	return drone
end

function Drone.create(pos, owner)
	game.effect("AddCritter", {
		id = UniqID.new(),
		pos = pos,
		vel = Vec2(0, 0),
		mass = 50,
		radius = 6,
		drag = 1 / 1.2, -- neutral buoyancy
		owner = owner,
		waterproof = false,
		texture = textures.get("drone"),
		state = Drone:new(pos),
		timer = 0,
	})
end

-- Count the number of drones deployed by this player in the area
function Drone.count(player, pos)
	local count = 0
	game.critters_iter(pos, 1000, 0, function(c)
		if c.owner == player and c.state.is_drone then
			count = count + 1
		end
	end)
	return count
end

return Drone
