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
local Forcefields = require("forcefields")
local Level = require("level")

-- Main entrypoint
-- This is called when initializing the game for a new round.
-- A fresh scripting environment is created for each round.
function luola_init_game(settings)
	-- Create a ship for each player
	for _, p in ipairs(settings.players) do
		local tpl = ships["vwing"].template
		game.effect(
			"AddShip",
			tableutils.combined(tpl, {
				pos = game.find_spawnpoint(),
				controller = p.controller,
				player = p.player,
				state = tableutils.combined(tpl.state, {
					on_fire_secondary = luola_secondary_weapons[p.weapon].fire_func,
				})
			})
		)

		game.player_effect("hud_overlay", p.player, {
			text = textures.font("menu", "Get ready!"),
			pos = Vec2(0.5, 0.1),
			color = game.player_color(p.player),
			lifetime = 3,
			fadeout = 1,
		})

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
	if settings["wind"] ~= false then
		Level.init_random_wind()
	end

	if settings["snowfall"] == true then
		Level.init_snowfall()
	end
end

-- Terrain explosion handler
-- This is called with a certain probability for each explosive pixel
-- when making a hole in the terrain.
function luola_explosive_terrain(pos, color)
	local tex = textures.get("pewpew")

	for a = 0, 360, (360 / 5) do
		game.effect("AddBullet", {
			pos = pos,
			vel = Vec2_for_angle(a + math.random(-30, 30), 1000.0),
			color = color,
			texture = tex,
			state = {
				on_impact = Impacts.bullet,
			},
		})
	end

	Impacts.make_firestarters(3, pos)
end

-- Splash handler is called when an object enters/exits water
function luola_splash(pos, vel, imass)
	local mag = vel:magnitude()
	if mag > 120 then
		for a = 0, 360, 10 do
			game.effect("AddTerrainParticle", {
				pos = pos + Vec2_for_angle(a, 6),
				vel = Vec2_for_angle(a, 300.0),
				color = game.water_color,
				imass = 1,
				drag = 0.002,
			})
		end
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
		title = "Claymore",
		fire_func = sweapons.landmine,
		description = "A remote detonable directional charge that can be placed on hard terrain. First trigger pull fires the mine from a rear facing launcher, second detonates.",
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
	shield = {
		title = "Shield",
		fire_func = sweapons.shield,
		description = "A grav-tech deflector shield that offers up to 99% protection against incoming fire.",
	},
	foam = {
		title = "Foam grenade",
		fire_func = sweapons.foam_grenade,
		description = "Originally developed as a firefighting tool, this weapon fires a glass sphere filled with pressurized foam that hardens in contact with air.",
	},
	greygoo = {
		title = "Grey goo",
		fire_func = sweapons.greygoo,
		description = "Universal self replicating nano-disassemblers. Each individual nanite contains a limiter to prevent out-of-control spread.",
	},
	freezer = {
		title = "Hailstone",
		fire_func = sweapons.freezer,
		description = "A glass sphere filled with liquid nitrogen. Can freeze a ship solid.",
	},
	nitroglycerin = {
		title = "Nitro-ampule",
		fire_func = sweapons.nitroglycerin,
		description = "A glass sphere filled with nitroglyserin. The soaked ground may detonate if disturbed by a sufficiently large shock.",
	},
	laser = {
		title = "Laser cannon",
		fire_func = sweapons.laser,
		description = "A directed energy weapon that hits targets at the speed of light.",
	}
}
