require "bullets"
require "utils.scheduler"
require "utils.trig"

function secondary_weapon_grenade(ship)
    local ammo = ship.ammo - 0.05
    if ammo < 0 then return end
    ship.ammo = ammo
    ship.secondary_weapon_cooldown = 0.4

    game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 1000.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("pewpew"),
		on_impact=grenade_impact,
	})	
end

function secondary_weapon_megabomb(ship)
    local ammo = ship.ammo - 0.1
    if ammo < 0 then return end
    ship.ammo = ammo
    ship.secondary_weapon_cooldown = 1.0

    game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel,
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("megabomb"),
		on_impact=megabomb_impact,
	})	
end

function secondary_weapon_rocket(ship)
    local ammo = ship.ammo - 0.1
    if ammo < 0 then return end
    ship.ammo = ammo
    ship.secondary_weapon_cooldown = 1.0

    game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 100.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("megabomb"),
		on_impact=megabomb_impact,
        state = {
            impulse = Vec2_for_angle(-ship.angle, 8000.0),
            scheduler = Scheduler:new():add(0, function(p)
                p:impulse(p.state.impulse)

                game.effect("AddParticle", {
                    pos = p.pos,
                    color = 0xffffffff,
                    target_color = 0x00ff0000,
                    lifetime = 0.15,
                    texture = textures.get("dot"),
                })

                return 0
            end)
        },
        timer = 0
	})	
end

function secondary_weapon_missile(ship)
    local ammo = ship.ammo - 0.1
    if ammo < 0 then return end
    ship.ammo = ammo
    ship.secondary_weapon_cooldown = 1.0

    game.effect("AddBullet", {
		pos = ship.pos,
		vel = ship.vel + Vec2_for_angle(-ship.angle, 100.0),
		mass = 300,
		radius = 5,
		drag = 0.0025,
		owner = ship.player,
		texture = textures.get("megabomb"),
		on_impact=missile_impact,
        state = {
            angle = -ship.angle,
            scheduler = Scheduler:new():add(0, function(this)
                local target = nil
                local nearest = 0

                game.ships_iter(function(ship)
                    local dist = ship.pos:dist(this.pos)
                    if ship.player ~= this.owner and (target == nil or dist < nearest) then
                        target = ship.pos
                        nearest = dist
                    end
                end)

                if target ~= nil then
                    local target_angle = -(target - this.pos):angle()
                    local turn = angle_difference(this.state.angle, target_angle)
                    if turn < 0 then
                        this.state.angle = this.state.angle - 20
                    else
                        this.state.angle = this.state.angle + 20
                    end
                    local impulse = Vec2_for_angle(this.state.angle, 10000)
                    this:impulse(impulse)
                
                    game.effect("AddParticle", {
                        pos = this.pos,
                        color = 0xffffffff,
                        target_color = 0x00ff0000,
                        lifetime = 0.15,
                        texture = textures.get("dot"),
                    })
                end

                return 0.02
            end)
        },
        timer = 0
	})	
end

function secondary_weapon_mine(ship)
	local ammo = ship.ammo - 0.1
	if ammo < 0 then return end
	ship.ammo = ammo
	ship.secondary_weapon_cooldown = 0.4

	local drag = 1 / 1.2
	if game.terrain_at(ship.pos) == 0x80 then
		drag = 1 / 60.0
	end

	game.effect("AddMine", {
		pos = ship.pos,
		vel = Vec2(0, 0),
		mass = 300,
		radius = 3,
		drag = drag,
		owner = ship.player,
		texture = textures.get("mine"),
		on_impact=grenade_impact,
		state = {
			scheduler = Scheduler:new():add(1, function(this)
				this.texture = textures.get("mine_active")
				this:disown()
			end)
		},
		timer=1
	})	
end
luola_secondary_weapons = {
    grenade = {
        title = "Grenade",
        fire_func = secondary_weapon_grenade,
    },
    megabomb = {
        title = "Megabomb",
        fire_func = secondary_weapon_megabomb,
    },
    rocket = {
        title = "Rocket launcher",
        fire_func = secondary_weapon_rocket,
    },
    missile = {
        title = "Homing missile",
        fire_func = secondary_weapon_missile,
    },
    mine = {
        title = "Mine",
        fire_func = secondary_weapon_mine,
    },
}
