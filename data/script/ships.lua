local Scheduler = require("utils.scheduler")
local weapons = require("primary_weapons")
local Impacts = require("weapons.impacts")
local Pilot = require("pilot")
local tableutils = require("utils.table")

local function vwing_thrust_effect(ship, uw, thrust)
	if uw then
		local a = math.floor(thrust * 0x66) << 24
		for i = 0, 5 do
			game.effect("AddParticle", {
				pos = ship.pos,
				vel = Vec2_for_angle(-ship.angle - 180 + math.random(-60, 60), 100),
				color = 0x00aaaaff | a,
				target_color = 0x00aaaaff,
				lifetime = 0.30,
				texture = textures.get("dot3x3"),
			})
		end
	else
		local a = math.floor(thrust * 0xff) << 24
		game.effect("AddParticle", {
			pos = ship.pos,
			vel = Vec2_for_angle(-ship.angle - 180, 300) + ship.vel,
			color = 0x00ffffff | a,
			target_color = 0x00ff0000,
			lifetime = 0.15,
			texture = textures.get("dot8x8"),
		})
	end
end

local function deltabomber_thrust_effect(ship, uw, thrust)
	local tex
	local a
	local vel
	if uw then
		a = math.floor(thrust * 0x66) << 24
		tex = textures.get("dot3x3")
		vel = Vec2_for_angle(-ship.angle - 180 + math.random(-60, 60), 100)
	else
		a = math.floor(thrust * 0xff) << 24
		tex = textures.get("dot8x8")
		vel = Vec2_for_angle(-ship.angle - 180, 300) + ship.vel
	end

	-- twin engines
	game.effect("AddParticle", {
		pos = ship.pos + Vec2_for_angle(-ship.angle - 180 - 35, 32),
		vel = vel,
		color = 0x00ffffff | a,
		target_color = 0x00ff0000,
		lifetime = 0.15,
		texture = tex,
	})

	game.effect("AddParticle", {
		pos = ship.pos + Vec2_for_angle(-ship.angle - 180 + 35, 32),
		vel = vel,
		color = 0x00ffffff | a,
		target_color = 0x00ff0000,
		lifetime = 0.15,
		texture = tex,
	})
	
end

local function ship_on_base(ship, timestep)
	if ship.cloaked or ship.state.forcefield ~= nil then
		-- pit crew can't see what they're doing
		-- (we don't want cloaked players to camp on bases indefinitely)
		return
	end
	local hp = ship.health
	ship:damage(-5 * timestep)

	if ship.health > hp then
		local r = ship.radius
		game.effect("AddParticle", {
			pos = ship.pos + Vec2(math.random() * r * 2 - r, math.random() * r * 2 - r),
			vel = Vec2(math.random(-60, 60), -160),
			a = Vec2(0, 9.8 * 50),
			color = 0xffffaa00,
			target_color = 0x00660000,
			lifetime = 1,
		})
	end
	ship.ammo = ship.ammo + timestep * 10
end

local function on_ship_destroyed(ship)
	-- We can't check this immediately on ship destruction
	-- because we need to check the state of all ships/players
	-- but changes to those haven't been committed yet.
	Scheduler.add_global(1, check_round_end_condition)

	game.effect("MakeBigHole", { pos = ship.pos, r = 16 })
	for i = 0, 2 do
		game.effect("AddParticle", {
			pos = ship.pos + Vec2(math.random(-30, 30), math.random(-30, 30)),
			texture = textures.get("bigboom"),
			reveal_in = i / 6,
		})
	end

	Impacts.make_shrapnell(36, ship.pos, {
		color = 0xffff6666,
		texture = textures.get("pewpew"),
		state = {
			on_impact = Impacts.bullet,
		}
	})
end

local function on_ship_eject(ship)
	Pilot.create(ship.pos, ship.player, ship.controller)
	ship.controller = 0
end

local function ship_touch_greygoo(ship)
	if ship.state.greygoo == nil then
		ship.state.greygoo = 15
		-- afflicted ship takes constant damage for a while and will also radiate short lived grey goo particles
		Scheduler.add_to_object(ship, 0.1, function(ship)
			game.player_effect("hud_overlay", ship.player, {
					text = textures.font("menu", "Nanite intrusion detected!"),
					pos = Vec2(0.5, 0.1),
					color = 0xffff0000,
					lifetime = 0.5,
					fadeout = 0.5,
			})

			ship:damage(1)
			ship.state.greygoo = ship.state.greygoo - 1
			Impacts.make_shrapnell(5, ship.pos, {
				mass = 50,
				radius = 5,
				drag = 0.0025,
				owner = ship.player,
				texture = textures.get("dot3x3"),
				color = 0x80ffffff,
				state = {
					scheduler = Scheduler.destroy_this,
					on_impact = Impacts.greygoo,
				},
				timer = 2/60,
			})
			if ship.state.greygoo > 0 then
				return 0.5
			else
				ship.state.greygoo = nil
			end
		end)
	end
end

local function ship_bullet_hit(ship, bullet, damage)
	ship:damage(damage)
end

local ships = {
	vwing = {
		title = "V-Wing",
		description = "An all-purpose fighter craft capable of operating in the atmosphere, underwater, and space.",
		template = {
			texture = textures.get("vwing"),
			mass = 1000,
			drag = 0.04,
			radius = 16,
			thrust = 40,
			turn_speed = 260,
			hitpoints = 100,
			state = {
				on_fire_primary = weapons.cannon,
				on_destroyed = on_ship_destroyed,
				on_base = ship_on_base,
				on_thrust = vwing_thrust_effect,
				on_touch_greygoo = ship_touch_greygoo,
				on_eject = on_ship_eject,
				on_bullet_hit = ship_bullet_hit,
			}
		},
	},
	deltabomber = {
		title = "Delta Bomber",
		description = "A heavy bomber that exchanges manoeuvrability for extra armor plating and cargo capacity.",
		template = {
			texture = textures.get("deltabomber"),
			mass = 3000,
			ammo = 160,
			drag = 0.05,
			radius = 18,
			thrust = 30,
			turn_speed = 220,
			hitpoints = 200,
			state = {
				on_fire_primary = weapons.delta_cannon,
				on_destroyed = on_ship_destroyed,
				on_base = ship_on_base,
				on_thrust = deltabomber_thrust_effect,
				on_touch_greygoo = ship_touch_greygoo,
				on_eject = on_ship_eject,
				on_bullet_hit = ship_bullet_hit,
			}
		},
	},
}

return ships
