local Scheduler = require("utils.scheduler")
local UniqID = require("utils.uniqid")
local Forcefields = require("forcefields")

local Gravmine = {}

function Gravmine.create(pos)
	game.effect("AddFixedObject", {
		pos = pos,
		texture = textures.get("dot8x8"), -- TODO nicer texture
		id = UniqID.new(),
		state = {
			scheduler = Scheduler:new()
				:add(1, Gravmine._activate)
				:add(30, function(obj)
					obj:destroy()
				end),a
		},
		on_destroy = Gravmine._on_destroy,
		timer = 1,
	})
end

function Gravmine._activate(obj)
	obj.state.forcefield = Forcefields.add({
		bounds = {obj.pos.x - 400, obj.pos.y - 400, 800, 800},
		point = 60,
	})
end

function Gravmine._on_destroy(obj)
	if obj.state.forcefield ~= nil then
		game.effect("RemoveForcefield", obj.state.forcefield)
	end
end

return Gravmine