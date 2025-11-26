local Scheduler = require("utils.scheduler")
local Impacts = require("weapons.impacts")
local Level = require("level")

local Bat = {}

function Bat._timer_fly(critter)
	if not critter.state.roosting then
		local speed = 50
		if critter.state.aggro > 0 then
			speed = 500
		end

		local delta = critter.state.target - critter.pos
		critter.vel = critter.vel + delta:normalized() * speed
		return 0.2
	end
end

function Bat._timer_decide_target(critter)
	critter.state.roost_counter = critter.state.roost_counter + 1

	if critter.state.roosting then
		-- If bat has rested enough, stop roosting and flying again
		if critter.state.roost_counter > 100 or critter.state.aggro > 0 then
			critter.state.roosting = false
			critter.state.roost_counter = 0
			Scheduler.add_to_object(critter, 0.2, Bat._timer_fly)
			critter.texture = textures.get("bat")
		else
			return 1
		end
	end

	if critter.state.aggro > 0 then
		critter.state.aggro = critter.state.aggro - 1
		local nearest_enemy_pos = nil
		local nearest_enemy_dist2 = 90000
		-- Bats have sonar and can find even cloaked ships
		game.ships_iter(function(ship)
			local dist2 = ship.pos:dist_squared(critter.pos)
			if dist2 < nearest_enemy_dist2 then
				nearest_enemy_pos = ship.pos
				nearest_enemy_dist2 = dist2
			end
		end)

		if nearest_enemy_pos ~= nil then
			critter.state.target = nearest_enemy_pos
			return 0.3
		end
	end

	if critter.state.roost_counter > 50 then
		-- Bat is getting tired, look for a place to roost
		local new_target = critter.pos - Vec2_for_angle(math.random(45, 135), 300)
		local _, _, hit = game.terrain_line(critter.pos, new_target)
		if hit then
			critter.state.target = new_target
			return 1
		end
	end

	for _ = 0, 8 do
		local new_target = critter.pos + Vec2_for_angle(math.random(0, 360), 100)
		local _, _, hit = game.terrain_line(critter.pos, new_target)
		if not hit then
			critter.state.target = new_target
			break
		end
	end
	return math.random(10) / 10
end

function Bat.on_touch_ground(critter)
	if not critter.state.roosting and game.terrain_at(critter.pos - Vec2(0, 3)) ~= 0 then
		critter.state.roosting = true
		critter.state.roost_counter = 0
		critter.texture = textures.get("bat_roosting")
	end
end

function Bat:new(pos)
	local bat = {
		scheduler = Scheduler:new():add(0, Bat._timer_fly):add(0, Bat._timer_decide_target),
		roost_counter = 0,
		roosting = false,
		aggro = 0,
	}
	setmetatable(bat, self)
	self.__index = self
	return bat
end

function Bat.on_bullet_hit(critter, bullet)
	if bullet.state ~= nil and bullet.state.is_nitro then
		bullet:destroy()
		critter.state.explosive = true
		critter.state.aggro = 40
		return false
	end

	if critter.state.aggro <= 0 then
		critter.state.aggro = 40
		return
	end

	critter:destroy()

	if critter.state.explosive then
		Impacts.grenade(critter, 0, nil)
	end

	local hit_angle = bullet.vel:angle()

	-- blood splatter
	for _ = 0, 4 do
		game.effect("AddTerrainParticle", {
			pos = critter.pos,
			vel = Vec2_for_angle(-hit_angle + math.random(-30, 30), 300.0),
			imass = 1,
			drag = 0.002,
			stain = true,
			color = 0x80ff0000,
		})
	end
end

function Bat.on_object_hit(critter, obj)
	if obj.is_ship then
		critter.state.aggro = 40
		game.player_effect("hud_overlay", obj.player, {
			texture = textures.get("bat_attack"),
			pos = Vec2(math.random(), math.random()),
			scale = 1,
			angle = math.random() * 360,
			lifetime = 1,
			fadeout = 0.4,
		})

		if critter.state.explosive then
			Impacts.grenade(critter, 0, obj)
		end
	end
end

function Bat.create(pos)
	game.effect("AddCritter", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 50,
		radius = 4,
		drag = 1 / 1.2, -- neutral buoyancy
		texture = textures.get("bat"),
		state = Bat:new(pos),
		timer = 0,
	})
end

function Bat.create_random(config)
	for _, area in ipairs(config) do
		for _ = 1, area["count"] do
			Bat.create(game.find_spawnpoint(Level.to_world_coordinates(area["area"]), false))
		end
	end
end

return Bat