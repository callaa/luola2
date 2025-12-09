local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")
local Level = require("level")

local Fish = {}

function Fish._timer_swim(critter)
	local delta
	if critter.state.east then
		delta = Vec2(30, math.random(-30, 30))
	else
		delta = Vec2(-30, math.random(-30, 30))
	end

	critter.vel = critter.vel + delta

	return 0.3
end

function Fish.on_touch_ground(critter)
	critter.state.east = not critter.state.east
	critter.vel = critter.vel * -1
end

function Fish.on_bullet_hit(critter, bullet)
	if bullet.state ~= nil and bullet.state.is_nitro then
		bullet:destroy()
		critter.state.explosive = true
		return true
	end

	if critter.state.explosive then
		Impacts.grenade(critter, 0, nil)
	end

	critter:destroy()

	-- blood splatter
	for a = 0, 360, (360 / 8) do
		game.effect("AddParticle", {
			pos = critter.pos,
			vel = Vec2_for_angle(a + math.random(-30, 30), 10.0),
			color = 0x80ff0000,
			target_color = 0x00ff0000,
			lifetime = 1 + math.random() * 3,
			texture = textures.get("dot8x8"),
		})
	end
end

function Fish:new(pos)
	local fish = {
		destination = pos,
		east = math.random() < 0.5,
		scheduler = Scheduler:new():add(0.1, Fish._timer_swim),
	}
	setmetatable(fish, self)
	self.__index = self
	return fish
end

function Fish.create(pos)
	game.effect("AddCritter", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 50,
		radius = 6,
		drag = 1 / 60.0, -- neutral buoyancy
		texture = textures.get("shark"),
		state = Fish:new(pos),
		timer = 0,
	})
end

-- confg: [ [count, [x, y, w, h]], ... ]
function Fish.create_random(config)
	for _, area in ipairs(config) do
		for _ = 1, area["count"] do
			Fish.create(game.find_spawnpoint(Level.to_world_coordinates(area["area"]), true))
		end
	end
end

return Fish
