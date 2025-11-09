local Scheduler = require("utils.scheduler")
local UniqID = require("utils.uniqid")
local Forcefields = require("forcefields")

local Grav = {}

function Grav.create_gravmine(pos)
	game.effect("AddFixedObject", {
		pos = pos,
		texture = textures.get("dot8x8"), -- TODO nicer texture
		id = UniqID.new(),
		state = {
			scheduler = Scheduler:new():add(1, Grav._activate_mine):add(30, Scheduler.destroy_this),
			a,
		},
		on_destroy = Grav._on_destroy,
		timer = 1,
	})
end

function Grav._activate_mine(obj)
	obj.state.forcefield = Forcefields.update({
		bounds = { obj.pos.x - 400, obj.pos.y - 400, 800, 800 },
		point = 60,
	})
end

function Grav._on_destroy(obj)
	if obj.state.forcefield ~= nil then
		game.effect("RemoveForcefield", obj.state.forcefield)
	end
end

function Grav.activate_shield(ship)
	ship.state.forcefield = UniqID.new()
	Scheduler.add_to_object(ship, 0, Grav._update_shield)
	Scheduler.add_to_object(ship, 0.1, Grav._consume_shield_energy)
end

function Grav.deactivate_shield(ship)
	game.effect("RemoveForcefield", ship.state.forcefield)
	ship.state.forcefield = nil
end

function Grav._consume_shield_energy(ship)
	if ship.state.forcefield ~= nil then
		ship.ammo = ship.ammo - 1
		if ship.ammo > 0 then
			return 0.1
		else
			Grav.deactivate_shield(ship)
		end
	end
end

function Grav._update_shield(ship)
	if ship.state.forcefield == nil then
		return
	end
	if ship.is_wrecked then
		Grav.deactivate_shield(ship)
		return
	end
	Forcefields.update({
		id = ship.state.forcefield,
		bounds = { ship.pos.x - 100, ship.pos.y - 100, 200, 200 },
		point = -500,
	})
	game.effect("AddParticle", {
		texture = textures.get("shield"),
		pos = ship.pos,
		lifetime = 0.1,
		target_color = 0x00ffffff,
	})
	return 0
end

return Grav
