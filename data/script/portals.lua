local Scheduler = require("utils.scheduler")

local Portals = {}

function Portals.create_portal_pair(source, destination)
	local tex = textures.get("portal")
	local lifetime = 4

	function teleport_object(portal, obj)
		obj.pos = (obj.pos - portal.pos) + destination
		return true -- ignore projectile's own hit handler
	end

	-- Entry portal
	game.effect("AddFixedObject", {
		pos = source,
		texture = tex,
		color = 0xfff0a422,
		radius = 16, 
		id = 0,
		state = {
			on_object_hit = teleport_object,
			on_bullet_hit = teleport_object,
			scheduler = Scheduler.destroy_this,
		},
		timer = lifetime
	})

	-- Exit portal (decorative)
	game.effect("AddParticle", {
		pos = destination,
		texture = tex,
		color = 0xff5fcde4,
		lifetime = lifetime,
	})
end

local function _portal_reminder(obj)
	game.player_effect("hud_overlay", obj.state.player, {
		texture = textures.get("portal_icon"),
		pos = Vec2(0, 0),
		align = "status",
		color = 0xa05fcde4,
		lifetime = 1.2,
		fadein = 0.6,
		fadeout = 0.6
	})
	return 1.2
end

function Portals.activate_jumpengine(ship)
	local found_destination = false
	game.fixedobjs_iter_mut(function(obj)
		if obj.state.is_portal and obj.state.player == ship.player then
			Portals.create_portal_pair(ship.pos, obj.pos)
			obj:destroy()
			found_destination = true
			return false
		end
	end)

	if not found_destination then
		game.effect("AddFixedObject", {
			pos = ship.pos,
			texture = textures.get("portal"),
			color = 0x105fcde4,
			id = 0,
			timer = 0,
			state = {
				is_portal = true,
				player = ship.player,
				scheduler = _portal_reminder,
			},
		})
	end
end

return Portals