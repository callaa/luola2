local Scheduler = require("utils.scheduler")

local snowfall = {}

function snowfall.init()
	local snow_zone = RectF(1, 1, game.level_width - 2, 10)

	Scheduler.add_global(1, function()
		for _ = 0, 10 do
			local p = game.find_spawnpoint(snow_zone)
			game.effect("AddTerrainParticle", {
				pos = p,
				vel=Vec2(0,0),
				terrain = 0x46,
				color = game.snow_color,
			})
		end
		return 2
	end)
end

return snowfall