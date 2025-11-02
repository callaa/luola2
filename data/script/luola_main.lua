-- This is the main entrypoint file for the game
-- By convention, functions and other values expected to be there
-- by the game engine are prefixed with "luola_" and are all collected
-- in this file.

local sweapons = require("secondary_weapons")
local bullets = require("bullets")
local ships = require("ships")
local tableutils = require("utils.table")
local Scheduler = require("utils.scheduler")
local Bird = require("critters.bird")
local Fish = require("critters.fish")
local Snowfall = require("snowfall")

-- Main entrypoint
-- This is called when initializing the game for a new round.
-- A fresh scripting environment is created for each round.
function luola_init_game(settings)
	-- Create a ship for each player
	for _, p in ipairs(settings.players) do
		game.effect(
			"AddShip",
			tableutils.combined(ships["vwing"].template, {
				pos = game.find_spawnpoint(),
				controller = p.controller,
				player = p.player,
				on_fire_secondary = luola_secondary_weapons[p.weapon].fire_func,
			})
		)
	end

	luola_init_level(settings.level)
end

-- Standard level initialization function
-- This is called indirectly by luola_init_game
-- This may be overridden in a level script to customize the level
function luola_init_level(settings)
	for k, v in pairs(settings) do
		print("Level setting:", k, v)
	end

	if settings["random-birds"] ~= nil then
		Bird.create_random(settings["random-birds"])
	end
	if settings["random-fish"] ~= nil then
		Fish.create_random(settings["random-fish"])
	end

	if settings["snowfall"] == true then
		Snowfall.init()
	end
end

-- Terrain explosion handler
-- This is called with a certain probability for each explosive pixel
-- when making a hole in the terrain.
function luola_explosive_terrain(x, y)
	local tex = textures.get("pewpew")
	local pos = Vec2(x, y)

	for a = 0, 360, (360 / 5) do
		game.effect("AddBullet", {
			pos = pos,
			vel = Vec2_for_angle(a + math.random(-30, 30), 1000.0),
			color = 0xffffa672,
			texture = tex,
			on_impact = bullets.bullet,
		})
	end
end

-- Game object timer callback
function luola_on_object_timer(obj, timestep)
	return obj.state.scheduler:service(obj, timestep)
end

-- Global scheduler timer callback
function luola_on_global_timer(timestep)
	return Scheduler._global:service(nil, timestep)
end

-- List of special weapons
-- This is referenced by the weapon selection screen
luola_secondary_weapons = {
	grenade = {
		title = "Grenade",
		fire_func = sweapons.grenade,
	},
	megabomb = {
		title = "Megabomb",
		fire_func = sweapons.megabomb,
	},
	rocket = {
		title = "Rocket launcher",
		fire_func = sweapons.rocket,
	},
	missile = {
		title = "Homing missile",
		fire_func = sweapons.missile,
	},
	mine = {
		title = "Mine",
		fire_func = sweapons.mine,
	},
	magmine = {
		title = "Magnetic mine",
		fire_func = sweapons.magmine,
	},
	landmine = {
		title = "Landmine",
		fire_func = sweapons.landmine,
	},
	drone = {
		title = "Drones",
		fire_func = sweapons.drone,
	},
	cloak = {
		title = "Chameleon skin",
		fire_func = sweapons.cloaking_device,
	},
}
