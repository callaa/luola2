local Utils = require("utils")

local dirt_texture = images.load_set("parts/dirt.toml").dirt
local rocks = Utils.partbuckets(images.load_set("parts/rocks.toml"))
local bases = Utils.partbuckets(images.load_set("parts/bases.toml"))
--debugparts = images.load_set("parts/debug.toml")

local water_level = level.height - level:randi(level.height / 6, level.height * 0.4)

-- Fill background
for y = 0, level.height, dirt_texture.height do
	for x = 0, level.width, dirt_texture.width do
		level:put_image(x, y, dirt_texture)
	end
end

-- Carve out the caverns
local cavern = images.new_hole_template(level.width, level.height)

local big_caves = level:rand_points(8)
for _, point in ipairs(big_caves) do
	-- draw a bunch of partially overlapping circles rather than one big one
	-- for a more organic/messy look
	for _, p in ipairs(level:rand_points_in({point.x - 80, point.y - 80, 160, 160}, 8)) do
		cavern:draw_circle(p, 30)
	end
end

-- Draw random tunnels
local points = { table.unpack(big_caves), table.unpack(level:rand_points(80)) }
local tunnels = Utils.spanning_tree(points)

for _, edge in ipairs(tunnels) do
	cavern:draw_line(edge[1], edge[2], 30)
	cavern:draw_circle(edge[1], level:randi(32, 48))
end

-- blur the hole template to smooth out the corners
-- and make the cutout
cavern:blur(20, 20, 2)
level:make_hole(0, 0, cavern)

-- Add some decorative rocks
for _, point in ipairs(level:rand_points(20)) do
	if Utils.check_all_solid_terrain(point, 10) then
		level:put_image(point.x, point.y, rocks.s:rand(), "center")
	end
end

-- Add bases to tunnel dead-ends
local function put_base_decorations(point)
	local points = {
		table.unpack(level:rand_points_in({point.x - 55, point.y, 40, 0}, 2)),
		table.unpack(level:rand_points_in({point.x + 15, point.y, 40, 0}, 2)),
	}

	for _, p in ipairs(points) do
		local floor = Utils.find_solid_ground(p, Vec2(0, 1), 60)
		if floor ~= nil then
			level:put_image(floor.x, floor.y + 2, bases.decor:rand(), "bottom-center", "under")
		end
	end
end

local base_count = 6
local potential_base_locations = Utils.find_dead_ends(tunnels)
while base_count > 0 and #potential_base_locations > 0 do
	local point = table.remove(potential_base_locations, level:randi(1, #potential_base_locations))
	if point.y > water_level then
		local roof = Utils.find_solid_ground(point, Vec2(0, -1), 60)
		if roof ~= nil then
			level:put_image(roof.x, roof.y - 2, bases.smalluwbase:rand(), "top-center", "under")
			base_count = base_count - 1
		elseif point.y - 60 < water_level then
			level:put_image(point.x, water_level, bases.floatbase:rand(), "center")
			base_count = base_count - 1
		end
	else
		local floor = Utils.find_solid_ground(point, Vec2(0, 1), 60)
		if floor ~= nil then
			if floor.y > water_level then
				level:put_image(floor.x, water_level, bases.floatbase:rand(), "center")
			else
				level:put_image(floor.x, floor.y + 2, bases.smallbase:rand(), "bottom-center", "under")
				put_base_decorations(point)
			end
			base_count = base_count - 1
		else
			if point.y + 60 > water_level then
				level:put_image(point.x, water_level, bases.floatbase:rand(), "center")
			end
		end
	end
end

-- Draw water
level:put_water({0, water_level, level.width, level.height}, 0.2)
