local Level = require("level")
local UniqID = require("utils.uniqid")

local Forcefields = {}

function Forcefields.update(ff)
	if ff.id == nil then
		ff.id = UniqID.new()
	end
	game.effect("UpdateForcefield", ff)
	return ff.id
end

function Forcefields.add_from_config(forcefields)
	for _, ff in ipairs(forcefields) do
		ff.bounds = Level.to_world_coordinates(ff.bounds)
		Forcefields.update(ff)
	end
end
return Forcefields
