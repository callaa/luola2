local Level = require("utils.level")

local Forcefields = {
	LAST_ID = 0,
}

function Forcefields.add(ff)
	if ff.id == nil then
		Forcefields.LAST_ID = Forcefields.LAST_ID + 1
		ff.id = Forcefields.LAST_ID
		game.effect("UpdateForcefield", ff)
	end
	return ff.id
end

function Forcefields.add_from_config(forcefields)
	for _, ff in ipairs(forcefields) do
		ff.bounds = Level.to_world_coordinates(ff.bounds)
		Forcefields.add(ff)
	end
end
return Forcefields
