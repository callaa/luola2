-- This is the main entrypoint file for the game
-- By convention, functions and other values expected to be there
-- by the game engine are prefixed with "luola_" and are all collected
-- in this file.

local tableutils = require("utils.table")
local Scheduler = require("utils.scheduler")
local sweapons = require("secondary_weapons")
local Impacts = require("weapons.impacts")
local ships = require("ships")
local Pilot = require("pilot")
local Bird = require("critters.bird")
local Bat = require("critters.bat")
local Fish = require("critters.fish")
local Spider = require("critters.spider")
local Forcefields = require("forcefields")
local Level = require("level")
local Turrets = require("turrets")

local player_settings = {}

-- Main entrypoint
-- This is called when initializing the game for a new round.
-- A fresh scripting environment is created for each round.
function luola_init_game(settings)
	for _, p in ipairs(settings.players) do
		player_settings[p.player] = p

		local pos = p.spawn
		if pos == nil then
			pos = game.find_spawnpoint()
		end

		if p.pilot_spawn ~= nil then
			Pilot.create(p.pilot_spawn, p.player, p.controller)
		end

		create_ship_for_player(p.player, pos, not p.pilot_spawn)

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

-- Create a new ship (global function)
function create_ship_for_player(player_id, pos, with_controller)
	local player = player_settings[player_id]
	local tpl = ships[player.ship].template
	local controller = player.controller
	if with_controller == false then
		controller = 0
	end

	game.effect(
		"AddShip",
		tableutils.combined(tpl, {
			pos = pos,
			controller = controller,
			player = player.player,
			state = tableutils.combined(tpl.state, {
				on_fire_secondary = luola_weapons[player.weapon].fire_func,
			})
		})
	)
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
	if settings["random-bats"] ~= nil then
		Bat.create_random(settings["random-bats"])
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
	if settings["turrets"] then
		Turrets.add_from_config(settings["turrets"])
	end
	if settings["wind"] ~= false then
		Level.init_random_wind()
	end

	if settings["snowfall"] == true then
		Level.init_snowfall()
	end

	-- Terrain (base) regeneration
	Scheduler.add_global(3, function()
		game.effect("RegenerateTerrain")
		return 3
	end)
end

-- Terrain explosion handler
-- This is called with a certain probability for each explosive pixel
-- when making a hole in the terrain.
function luola_explosive_terrain(pos, color)
	local tex = textures.get("pewpew")

	game.effect("AddParticle", {
		pos = pos,
		texture = textures.get("bigboom"),
	})

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

-- End the round if end condition holds
function check_round_end_condition()
	local last_player_standing = 0
	local count = 0

	game.ships_iter(function(ship)
		if ship.controller ~= 0 then
			count = count + 1
			last_player_standing = ship.player
		end
	end)

	game.pilots_iter(function(pilot)
		count = count + 1
		last_player_standing = pilot.player
	end)

	if count <= 1 then
		game.effect("EndRound", last_player_standing)
	end
end

-- Game object timer callback
function luola_on_object_timer(obj, timestep)
	return obj.state.scheduler(obj, timestep)
end

-- Global scheduler timer callback
function luola_on_global_timer(timestep)
	return Scheduler._global:service(nil, timestep)
end

-- List of special weapons
-- This is referenced by the weapon selection screen and luola_init_game()
luola_weapons_default = "grenade"
luola_weapons = {
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
	moving_gravmine = {
		title = "Gravity mine (unbalanced)",
		fire_func = sweapons.moving_gravmine,
		description = "A variant of the gravity mine. A deliberately engineered inbalance in the field causes the anomaly to move in a straight line.",
	},
	drone = {
		title = "Drone (flying)",
		fire_func = sweapons.drone,
		description = "An autonomous target seeking drone equipped with a rapid-fire cannon and a payload capacity of up to 30 armor piercing rounds. Due to signal interference, only a limited number of drones can be deployed in an area.",
	},
	tank = {
		title = "Drone (wheeled)",
		fire_func = sweapons.tank,
		description = "A wheeled autonomous munition delivery platform. Compared to flying drones, these ground based units can carry much heavier weaponry.",
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
		description = "A glass sphere filled with nitroglycerin. The soaked ground may detonate if disturbed by a sufficiently large shock.",
	},
	laser = {
		title = "Laser cannon",
		fire_func = sweapons.laser,
		description = "A directed energy weapon that hits targets at the speed of light.",
	},
	digger = {
		title = "Sonic chisel",
		fire_func = sweapons.diggerbeam,
		description = "Emits an ultrasonic beam that breaks down rock and loosens dirt material. Primarily a digging tool; not very effective against modern armor.",
	},
	chemtrail = {
		title = "Chemtrail dispenser",
		fire_func = sweapons.chemtrail,
		description = "Releases a toxic mist behind the ship.",
	},
	jumpengine = {
		title = "Jump engine",
		fire_func= sweapons.jumpengine,
		description = "Generates a wormhole allowing instantaneous travel across any distance.",
	},
}

-- List of selectable ships
-- This is used in the ship/weapon selection screen
luola_ships = {}
luola_ships_default = "vwing"
for name, ship in pairs(ships) do
	luola_ships[name] = {
		title = ship.title,
		description = ship.description,
		texture = ship.template.texture,
	}
end
