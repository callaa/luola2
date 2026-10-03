local Utils = require("utils")

local asteroids = Utils.partbuckets(images.load_set("parts/asteroids.toml"))
local basestations = Utils.partbuckets(images.load_set("parts/basestations.toml"))
-- debugparts = images.load_set("parts/debug.toml")

--level:rand_set_seed(1)

-- A set of random points on the level where asteroids can be added
local points = level:rand_points(120)
local used_points = {}

for bucket_name, asteroid_count in pairs(level.config.asteroids) do
	local bucket = asteroids[bucket_name]

	for _ = 1, asteroid_count do
		local asteroid = bucket:rand()
		local radius = math.max(asteroid.width, asteroid.height) * 0.6
		local point = Utils.take_random_point_with_spacing(points, used_points, radius)
		if point == nil then
			print("Out of points, can't add asteroid size", bucket_name)
			break
		end

		level:put_image(point.x, point.y, asteroid, "center")
		--level:put_image(point.x, point.y, debugparts.blue_cross, "center")
		Utils.remove_nearby_points(points, point, radius)
	end
end

-- Add bases

level.settings.gravity_generators = {}

for _ = 1, 4 do
	local base = basestations.base:rand()
	local _, point = Utils.take_random_item(points)
	if point == nil then
		print("No room for more bases!")
		break
	end
	level:put_image(point.x, point.y, base, "center")
	table.insert(level.settings.gravity_generators, {
		point.x, point.y, math.max(base.width, base.height) / 2,
	})
end

-- Debug: show leftover points
--for _, p in ipairs(points) do
--	level:put_image(p.x, p.y, debugparts.green_cross, "center")
--end