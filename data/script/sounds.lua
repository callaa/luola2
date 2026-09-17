-- Return a random sound from the set
local function s(...sounds)
	if #sounds == 1 then
		local sound = sfx.get(sounds[1])
		if not sound.is_valid then
			print("Warning: couldn't find sound effect", sounds[1])
		end
		return function() return sound end
	else
		local set = {}
		for _, sound in ipairs({ ... }) do
			local s = sfx.get(sound)
			if s.is_valid then
				table.insert(set, sfx.get(sound))
			else
				print("Warning: couldn't find sound effect", sound)
			end
		end

		return function()
			return set[math.random(1, #set)]
		end
	end
end


return {
	thump = s("thump"),
	click = s("stone-click"),
	small_explosion = s("small-explosion"),
	big_explosion = s("big-explosion"),
	freezings = s("freezing", "freezing2"),
	glass_break = s("glass-break"),
	grenade_launcher = s("grenade-launcher"),
	big_launcher = s("big-launcher"),
	launcher = s("launcher"),
	repair = s("repair1", "repair2", "repair3"),
	high_warble = s("high-warble"),
	low_warble = s("low-warble"),
	laser = s("laser"),
	hull_impact = s("metal-impact", "metal-impact1", "metal-impact2"),
	gunshot = s("gunshot"),
	bat_chirp = s("bat-chirp"),
	bats = s("bats1", "bats2", "bats3"),
	bird = s("bird-caw"),
	warp = s("warp"),
	ping = s("ping"),
	drone_pursuing = s("drone-pursuing"),
	drone_roaming = s("drone-roaming"),
	sand = s("sand"),
	foam = s("foam"),
}