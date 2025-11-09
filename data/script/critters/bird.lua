local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")

local Bird = {}

function Bird._timer_flock(critter)
	local center = Vec2(0, 0)
	local boid_count = 0
	local avoid = Vec2(0, 0)
	local vel_match = Vec2(0, 0)
	game.critters_iter(critter.pos, 300, critter.id, function(other)
		if not other.state.is_bird then
			return
		end
		boid_count = boid_count + 1
		center = center + other.pos
		local dd = critter.pos:dist_squared(other.pos)
		if dd < 100 * 100 then
			avoid = avoid + (critter.pos - other.pos)
		end
		vel_match = vel_match + other.vel
	end)

	if boid_count > 0 then
		center = center / boid_count
		vel_match = vel_match / boid_count

		-- Rule 1: move towards the center of the flock
		-- Rule 2: avoid getting too close
		-- Rule 3: match velocities
		critter.vel = critter.vel + ((center - critter.pos) * 0.2 + avoid * 0.2) + vel_match * 0.4
	end

	-- Extra rule: avoid solid terrain
	for a = 0, 360, 60 do
		local danger, t, _ = game.terrain_line(critter.pos, critter.pos + Vec2_for_angle(a, 60))
		if t ~= 0 then
			critter.vel = critter.vel + (critter.pos - danger) * 0.8
		end
	end

	return 0.2
end

function Bird._on_bullet_hit(critter, bullet)
	if bullet.state ~= nil and bullet.state.is_nitro then
		bullet:destroy()
		critter.state.explosive = true
		return false
	end

	critter:destroy()

	if critter.state.explosive then
		Impacts.grenade(critter, 0, nil)
	end

	local hit_angle = bullet.vel:normalized():angle()

	-- blood splatter
	for _ = 0, 4 do
		game.effect("AddTerrainParticle", {
			pos = critter.pos,
			vel = Vec2_for_angle(-hit_angle + math.random(-30, 30), 300.0),
			imass = 1,
			drag = 0.002,
			stain = true,
			color = 0x80ff0000,
		})
	end

	-- puff of feathers
	for a = 0, 360, (360 / 20) do
		game.effect("AddTerrainParticle", {
			pos = critter.pos,
			vel = Vec2_for_angle(a + math.random(-10, 10), 400.0),
			color = 0x99ffffff,
		})
	end
end

function Bird:new(pos)
	local bird = {
		is_bird = true,
		scheduler = Scheduler:new():add(0, Bird._timer_flock),
	}
	setmetatable(bird, self)
	self.__index = self
	return bird
end

function Bird.create(pos)
	game.effect("AddCritter", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 50,
		radius = 4,
		drag = 1 / 1.2, -- neutral buoyancy
		texture = textures.get("bird"),
		state = Bird:new(pos),
		on_bullet_hit = Bird._on_bullet_hit,
		timer = 0,
	})
end

function Bird.create_random(count)
	local flocks = math.ceil(count / 5)
	local birds_in_flock = count // flocks

	for _ = 0, flocks do
		local center = game.find_spawnpoint(nil, false)
		local area = RectF(center.x - 300, center.y - 300, 600, 600)
		for _ = 0, birds_in_flock do
			Bird.create(game.find_spawnpoint(area, false))
		end
	end
end

return Bird
