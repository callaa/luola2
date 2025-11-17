local Scheduler = require("utils.scheduler")
local tableutils = require("utils.table")

local function bullet_hell_impact(this, terrain, obj)
	this:destroy()
	game.effect("MakeBulletHole", this.pos)
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("boom"),
	})

	if obj ~= nil and obj.damage ~=nil then
		obj:damage(0.1)
	end
end

local function bullet_hell(bullet_count)
	for i = 0, bullet_count do
		-- Note: we use AddMine here instead of AddBullet as a performance test.
		-- Mine type projectiles can collide with other projectiles and are therefore
		-- more expensive to compute.
		game.effect("AddMine", {
			pos = game.find_spawnpoint(),
			vel = Vec2(math.random() * 300, math.random() * 300),
			texture = textures.get("pewpew"),
			on_impact = bullet_hell_impact,
		})
	end
	return 1
end

local original_init_level = luola_init_level
function luola_init_level(settings)
	original_init_level(settings)

	-- run this function 1 second from now
	Scheduler.add_global(1, function()
		bullet_hell(settings.bullets_per_second)

		-- rerun after one second
		return 1
	end)
end
