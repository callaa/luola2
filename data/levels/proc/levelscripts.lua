local UniqID = require("utils.uniqid")
local Level = require("level")
local Forcefields = require("forcefields")

local function gravgen_bullet_hit(obj)
	obj:destroy()
	game.effect("RemoveForcefield", obj.state.forcefield)
end

local function create_gravity_generator(x, y, radius)
	game.effect("AddFixedObject", {
		pos = Vec2(x, y),
        texture = textures.get("lvl_grav"),
		radius = 5,
        id = UniqID.new(),
        state = {
			on_bullet_hit = gravgen_bullet_hit,
			forcefield = Forcefields.update({
				bounds = { x - radius, y - radius, radius * 2, radius },
				uniform = {0, 10}
			})
        },
    })
end

local original_init_level = luola_init_level
function luola_init_level(settings)
	original_init_level(settings)

	-- gravity generators hold the ship on the base while recharging.
	-- the bases are destructible, so these must be too.
	for _, gg in ipairs(settings.gravity_generators) do
		create_gravity_generator(table.unpack(Level.to_world_coordinates(gg)))
	end
end