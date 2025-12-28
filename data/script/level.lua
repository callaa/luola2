local Scheduler = require("utils.scheduler")

-- These should be kept in sync with the values in terrain.rs
local Level = {
	TER_TYPE_GROUND = 1,
	TER_TYPE_BURNABLE = 2,
	TER_TYPE_CINDER = 3,
	TER_TYPE_EXPLOSIVE = 4,
	TER_TYPE_HIGH_EXPLOSIVE = 5,
	TER_TYPE_ICE = 6,
	TER_TYPE_BASE = 7,
	TER_TYPE_BASE_NOREGEN = 8,
	TER_TER_TYPE_WALKWAY = 10,
	TER_TYPE_GREYGOO = 11,
	TER_TYPE_DAMAGE = 12,
	TER_LEVELBOUND = 0x3f,
}

function Level.mask_solid(ter)
	return ter & 0x1f
end

function Level.is_water(ter)
	return ter & 0xbf == 0x80
end

function Level.is_indestructible(ter)
	return (ter & 0x40) == 0
end

function Level.is_burnable(ter)
	local solid = Level.mask_solid(ter)
	return solid == Level.TER_TYPE_BURNABLE or solid == Level.TER_TYPE_CINDER
end

function Level.is_explosive(ter)
	local solid = Level.mask_solid(ter)
	return solid == Level.TER_TYPE_EXPLOSIVE or solid == Level.TER_TYPE_HIGH_EXPLOSIVE
end

function Level.is_base(ter)
	local solid = Level.mask_solid(ter)
	return solid == Level.TER_TYPE_BASE or solid == Level.TER_TYPE_BASE_NOREGEN
end

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
