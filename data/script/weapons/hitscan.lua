local Hitscan = {}

local function laser_hit_object(laser, obj)
	if obj.is_ship then
		obj:damage(5)
	elseif obj.is_critter then
		obj.state.on_bullet_hit(obj, laser)
	elseif obj.state.on_impact ~= nil then
		obj.state.on_impact(obj, game.terrain_at(obj.pos), nil)
	end
end

function Hitscan.laser(pos, angle, owner)
	game.effect("AddHitscan", {
		start = pos,
		stop = pos + Vec2_for_angle(-angle, 3000),
		owner = owner,
		state = {
			is_laser = true,
			on_hit_object = laser_hit_object,
			on_done = function(hs)
				if hs.terrain == 0x42 or hs.terrain == 0x43 then
						game.effect("AddDynamicTerrain", {
						pos = hs.stop,
						type = "Fire",
					})
				elseif hs.terrain == 0x44 or hs.terrain == 0x45 then
					luola_explosive_terrain(hs.stop, 0xffff0000)
				end

				local tex = textures.get("dot3x3")
				local len = hs.start:dist(hs.stop)
				local step = Vec2_for_angle(-angle, 1)
				for i = 0,len,3 do
					local p = pos + step * i
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

return Hitscan