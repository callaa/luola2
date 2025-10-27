require "utils.table"

function make_shrapnell(count, pos, template)
    for a = 0, 360, (360 / count) do
        game.effect("AddBullet", combined_tables(
            template,
            {
                pos = pos + Vec2_for_angle(a, 3.0),
                vel = Vec2_for_angle(a, 1000.0),
            }
        ))
    end
end

function bullet_impact(this, terrain, ship)
	this:destroy()
	game.effect("MakeBulletHole", this.pos)
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("boom"),
	})

    if ship ~= nil then
        ship:damage(3)
    end
end

function grenade_impact(this, terrain, ship)
	this:destroy()
	game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

    if ship ~= nil then
        ship:damage(1)
    end

    make_shrapnell(36, this.pos, {
        color = 0xffff6666,
        mass = 30,
        radius = 1,
        drag = 0.0025,
        texture = textures.get("pewpew"),
        on_impact = bullet_impact,
    })
end

function megabomb_impact(this, terrain, ship)
    this:destroy()
    game.effect("MakeBigHole", { pos = this.pos, r = 16 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

    if ship ~= nil then
        ship:damage(20)
    end

    make_shrapnell(10, this.pos, {
        mass = 300,
        radius = 5,
        drag = 0.0025,
        texture = textures.get("pewpew"),
        on_impact = grenade_impact,
    })
end

function rocket_impact(this, terrain, ship)
    this:destroy()
    game.effect("MakeBigHole", { pos = this.pos, r = 12 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

    if ship ~= nil then
        ship:damage(15)
    end

    make_shrapnell(4, this.pos, {
        mass = 300,
        radius = 5,
        drag = 0.0025,
        texture = textures.get("pewpew"),
        on_impact = grenade_impact,
    })
end

function missile_impact(this, terrain, ship)
    this:destroy()
    game.effect("MakeBigHole", { pos = this.pos, r = 8 })
	game.effect("AddParticle", {
		pos = this.pos,
		texture = textures.get("bigboom"),
	})

    if ship ~= nil then
        ship:damage(10)
    end

    make_shrapnell(20, this.pos, {
        color = 0xffff6666,
        mass = 30,
        radius = 1,
        drag = 0.0025,
        texture = textures.get("pewpew"),
        on_impact = bullet_impact,
    })
end