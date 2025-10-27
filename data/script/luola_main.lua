require "secondary_weapons"
require "ships"
require "utils.table"
require "utils.scheduler"

-- Game initialization function
function luola_init_game(settings)
	-- Create a ship for each player
	for _, p in ipairs(settings.players) do
		game.effect("AddShip", combined_tables(
			luola_ships["vwing"].template,
			{
				pos = game.find_spawnpoint(),
				controller = p.controller,
				player = p.player,
				on_fire_secondary = luola_secondary_weapons[p.weapon].fire_func,
			}
		))
	end

	luola_init_level(settings.level)
end

-- Standard level initialization function
-- This may be overridden in a level script to customize the level
function luola_init_level(settings)
	for k, v in pairs(settings) do
		print("Level setting:", k, v)
	end
end

