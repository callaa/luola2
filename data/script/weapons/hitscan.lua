local Level = require("level")
local Hitscan = {}

local function laser_hit_object(laser, obj)
	if obj.state.on_bullet_hit then
		obj.state.on_bullet_hit(obj, laser, 5)
	end
end

function Hitscan.laser(pos, angle, owner)
	Hitscan.laser_to(
		pos,
		pos + Vec2_for_angle(-angle, 3000),
		owner
	)
end

function Hitscan.laser_to(start, stop, owner)
	game.effect("AddHitscan", {
		start = start,
		stop = stop,
		owner = owner,
		state = {
			is_laser = true,
			on_hit_object = laser_hit_object,
			on_done = function(hs)
				if Level.is_burnable(hs.terrain) then
						game.effect("AddDynamicTerrain", {
						pos = hs.stop,
						type = "Fire",
					})
				elseif Level.is_explosive(hs.terrain) then
					luola_explosive_terrain(hs.stop, 0xffff0000)
				end

				local tex = textures.get("dot3x3")
				local len = hs.start:dist(hs.stop)
				local step = (hs.stop - start) / len
				for i = 0,len,3 do
					local p = hs.start + step * i
					game.effect("AddParticle", {
						pos = p,
						texture = tex,
						color = 0xffff6666,
						target_color = 0x00ff0000,
						lifetime = 3/60,
					})

					-- Create bubbles underwater
					if game.terrain_at(p) & 0x80 > 0 then
						game.effect("AddParticle", {
							pos = p,
							a = Vec2(0, -100),
							wind = true,
							color = 0x80ffffff,
							target_color = 0x00ffffff,
							lifetime = 2,
						})
					end
				end
			end
		},
	})
end

function Hitscan.deathray(pos, angle, owner)
	Hitscan.deathray_to(
		pos,
		pos + Vec2_for_angle(-angle, 2000),
		owner
	)
end

function Hitscan.deathray_to(start, stop, owner)
	game.effect("AddHitscan", {
		start = start,
		stop = stop,
		owner = owner,
		hit_multiple = true,
		state = {
			is_laser = true,
			on_hit_object = laser_hit_object,
			on_done = function(hs)
				if Level.is_burnable(hs.terrain) then
						game.effect("AddDynamicTerrain", {
						pos = hs.stop,
						type = "Fire",
					})
				elseif Level.is_explosive(hs.terrain) then
					luola_explosive_terrain(hs.stop, 0xffff0000)
				end

				local tex = textures.get("dot3x3")
				local len = hs.start:dist(hs.stop)
				local step = (hs.stop - start) / len
				for i = 0,len,4 do
					local p = hs.start + step * i
					game.effect("AddParticle", {
						pos = p,
						texture = tex,
						vel = Vec2(math.random() - 0.5, math.random() - 0.5) * 30,
						color = 0xff6666ff,
						target_color = 0x000000ff,
						lifetime = 0.8,
					})
				end
			end
		},
	})
end

return Hitscan