-- This is the main entrypoint file for the game
-- By convention, functions and other values expected to be there
-- by the game engine are prefixed with "luola_" and are all collected
-- in this file.

local sweapons = require("secondary_weapons")
local Impacts = require("weapons.impacts")
local ships = require("ships")
local tableutils = require("utils.table")
local Scheduler = require("utils.scheduler")
local Bird = require("critters.bird")
local Fish = require("critters.fish")
local Spider = require("critters.spider")
local Snowfall = require("snowfall")
local Forcefields = require("forcefields")

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
	if settings["random-spiders"] ~= nil then
		Spider.create_random(settings["random-spiders"])
	end
	if settings["forcefields"] ~= nil then
		Forcefields.add_from_config(settings["forcefields"])
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
			on_impact = Impacts.bullet,
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
		description = "The grenade launcher fires a lightweight fragmentation grenade that can deal a surprising amount of damage for its size.",
	},
	megabomb = {
		title = "Megabomb",
		fire_func = sweapons.megabomb,
		description = "An unguided bomb packed full of high explosives for massive damage.",
	},
	rocket = {
		title = "Rocket launcher",
		fire_func = sweapons.rocket,
		description = "Though smaller than the Megabomb, this self propelled weapon can still carry a large explosive payload.",
	},
	missile = {
		title = "Homing missile",
		fire_func = sweapons.missile,
		description = "The addition of a guidance system has reduced the available payload capacity but the autonomous target seeking capability makes up for it.",
	},
	mine = {
		title = "Mine",
		fire_func = sweapons.mine,
		description = "A floating mine with variable buoyancy suitable for use in both water and open air.",
	},
	magmine = {
		title = "Magnetic mine",
		fire_func = sweapons.magmine,
		description = "A mine augmented with a short range magnetic target seeking system.",
	},
	landmine = {
		title = "Landmine",
		fire_func = sweapons.landmine,
		description = "A remote detonable directional charge that can be placed on hard terrain.",
	},
	gravmine = {
		title = "Gravity mine",
		fire_func = sweapons.gravmine,
		description = "Generates a short-lived artificial gravity well far deeper than the device's own mass-energy would permit according to classical physics.",
	},
	drone = {
		title = "Drones",
		fire_func = sweapons.drone,
		description = "Autonomous target seeking drones equipped with a rapid-fire cannon.",
	},
	tank = {
		title = "Tank",
		fire_func = sweapons.tank,
		description = "A wheeled autonomous destruction delivery platform. Compared to the flying drone, this ground based unit can carry much heavier weaponry.",
	},
	cloak = {
		title = "Chameleon skin",
		fire_func = sweapons.cloaking_device,
		description = "Active optical surface coating that can render the ship nearly invisible.",
	},
	ghostship = {
		title = "Improbability drive",
		fire_func = sweapons.ghostship,
		description = "A quantum mechanical device that alters the natural probability field around the ship, allowing it to pass through solid ground.",
	},
}
