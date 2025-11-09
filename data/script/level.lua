local Scheduler = require("utils.scheduler")
local Level = {}

-- Convert a list of coordinates from level to world coordinates
function Level.to_world_coordinates(coordinates)
	local w = {}
	for _, c in ipairs(coordinates) do
		table.insert(w, c * 3)
	end
	return w
end

local TARGET_WINDSPEED = 0
local WINDSPEED = 0
local function change_wind()
	local sign = 1
	if WINDSPEED < 0 then
		sign = -1
	end

	if math.random() < 0.3 then
		sign = -sign
	end

	TARGET_WINDSPEED = math.random() * sign

	return 1 + math.random() * 15
end

local function update_windspeed()
	local new_speed = WINDSPEED + (TARGET_WINDSPEED - WINDSPEED) / 10
	if math.abs(new_speed - WINDSPEED) > 0.01 then
		game.effect("SetWindspeed", new_speed)
		WINDSPEED = new_speed
	end
	return 0.1
end

function Level.init_random_wind()
	Scheduler.add_global(0, change_wind)
	Scheduler.add_global(0.1, update_windspeed)
end

function Level.init_snowfall()
	local snow_zone = RectF(1, 1, game.level_width - 2, 10)

	Scheduler.add_global(1, function()
		for _ = 0, 10 do
			local p = game.find_spawnpoint(snow_zone)
			game.effect("AddTerrainParticle", {
				pos = p,
				vel = Vec2(0, 0),
				terrain = 0x46,
				color = game.snow_color,
			})
		end
		return 2
	end)
end

return Level
