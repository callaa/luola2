local Impacts = require("weapons.impacts")
local Scheduler = require("utils.scheduler")

local mines = {}

-- A regular mine.
-- Disowns itself after a second and becomes dangerous even
-- to the original player
function mines.create_mine(pos, owner)
	local drag = 1 / 1.2
	if game.terrain_at(pos) == 0x80 then
		drag = 1 / 60.0
	end

	game.effect("AddMine", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 300,
		radius = 3,
		drag = drag,
		owner = owner,
		texture = textures.get("mine"),
		on_impact = Impacts.grenade,
		state = {
			scheduler = Scheduler:new():add(1, function(this)
				this.texture = textures.get("mine_armed")
				this:disown()
			end),
		},
		timer = 1,
	})
end

local function magmine_attract_timer(this)
	local nearest_enemy_pos = nil
	local nearest_enemy_dist2 = 300 * 300

	game.ships_iter(function(ship)
		if ship.player ~= this.owner then
			local dist2 = ship.pos:dist_squared(this.pos)
			if dist2 < nearest_enemy_dist2 then
				nearest_enemy_pos = ship.pos
				nearest_enemy_dist2 = dist2
			end
		end
	end)

	if nearest_enemy_pos ~= nil then
		local a = (nearest_enemy_pos - this.pos):normalized() * (50000 / math.sqrt(nearest_enemy_dist2))
		this.vel = this.vel + a
		return 0.1
	end

	return 0.6
end

-- A magnetic mine that is attracted to nearby ships
function mines.create_magmine(pos, owner)
	local drag = 1 / 1.2
	if game.terrain_at(pos) == 0x80 then
		drag = 1 / 60.0
	end

	game.effect("AddMine", {
		pos = pos,
		vel = Vec2(0, 0),
		mass = 300,
		radius = 8,
		drag = drag,
		owner = owner,
		texture = textures.get("magmine"),
		on_impact = Impacts.grenade,
		state = {
			scheduler = Scheduler:new()
				:add(1, function(this)
					this.texture = textures.get("magmine_armed")
					this:disown()
				end)
				:add(1.1, magmine_attract_timer),
		},
		timer = 1,
	})
end

local function detonate_landmine(mine)
	mine:destroy()
	game.effect("MakeBigHole", { pos = mine.pos, r = 6 })
	game.effect("AddParticle", {
		pos = mine.pos,
		texture = textures.get("bigboom"),
	})

	local tex = textures.get("pewpew")
	for a = -15, 15, 2 do
		game.effect("AddBullet", {
			pos = mine.pos + Vec2_for_angle(mine.state.angle + a, 1.0),
			vel = Vec2_for_angle(mine.state.angle + a, 1500.0),
			texture = tex,
			on_impact = Impacts.bullet,
		})
	end
end

function mines.detonate_landmine(player)
	local detonated = false
	-- Detonate existing mine (if exists)
	game.mines_iter_mut(owner, function(mine)
		if mine.state.is_landmine then
			detonate_landmine(mine)
			detonated = true
			return false
		end
	end)

	return detonated
end

function mines.create_landmine(pos, angle, owner)
	game.effect("AddMine", {
		pos = pos,
		vel = Vec2_for_angle(-angle + 180, 1000),
		mass = 300,
		radius = 0,
		drag = drag,
		owner = owner,
		-- terrain_collision = "simple",
		texture = textures.get("dot3x3"),
		state = {
			angle = -angle,
			is_landmine = true,
		},
	})
end

return mines
