local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")
local Level = require("level")

local Spider = {}

function Spider._on_touch_ledge(critter)
	critter.walking = -critter.walking
end

function Spider._on_touch_ground(critter)
	if critter.state.slinging and not critter.rope_attached then
		critter.state.slinging = false
		critter.walking = 1
		critter.texture = textures.get("spider")
		Scheduler.add_to_object(critter, math.random(6, 16), Spider._try_ceiling)
	end
end

-- Check if there is a ceiling low enough to hang from
function Spider._try_ceiling(critter)
	local pos, t, hit = game.terrain_line(critter.pos, critter.pos + Vec2(0, -200))
	if hit then
		critter.walking = 0
		critter.texture = textures.get("spider_hanging")
		critter.state.slinging = true
		critter:attach_rope(pos)
		Scheduler.add_to_object(critter, 0.2, Spider._climb_thread)
		return
	end
	return math.random(6, 16)
end

-- Climb the rope up and down
function Spider._climb_thread(critter)
	if critter.rope_length < 40 then
		return nil
	end
	critter:climb_rope(-3)

	return 0.1
end

function Spider._on_bullet_hit(critter, bullet)
	if bullet.state ~= nil and bullet.state.is_nitro then
		bullet:destroy()
		critter.state.explosive = true
		return false
	end

	if critter.state.explosive then
		Impacts.grenade(critter, 0, nil)
	end

	if critter:detach_rope() then
		return
	end

	critter:destroy()

	-- blood splatter
	for a = 0, 360, (360 / 16) do
		game.effect("AddTerrainParticle", {
			pos = critter.pos,
			vel = Vec2_for_angle(a + math.random(-30, 30), 300.0),
			color = 0x80c5be02,
			imass = 1,
			drag = 0.002,
			stain = true,
		})
	end
end

function Spider:new(pos)
	local spider = {
		scheduler = Scheduler:new():add(math.random(6, 16), Spider._try_ceiling),
	}
	setmetatable(spider, self)
	self.__index = self
	return spider
end

function Spider.create(pos)
	game.effect("AddCritter", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 50,
		radius = 6,
		walking = 1,
		drag = 0.1,
		texture = textures.get("spider"),
		state = Spider:new(pos),
		on_bullet_hit = Spider._on_bullet_hit,
		on_touch_ledge = Spider._on_touch_ledge,
		on_touch_ground = Spider._on_touch_ground,
		timer = 0,
	})
end

-- confg: [ [count, [x, y, w, h]], ... ]
function Spider.create_random(config)
	for _, area in ipairs(config) do
		for _ = 1, area["count"] do
			Spider.create(game.find_spawnpoint(Level.to_world_coordinates(area["area"]), false))
		end
	end
end

return Spider
