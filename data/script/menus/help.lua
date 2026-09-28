local BLIP0 = sfx.get("blip0")
local BLIP1 = sfx.get("blip1")
local BLIP2 = sfx.get("blip2")

local function controls_guide(caption, image)
    sfx.blip(BLIP2)
    return Action.Push(Menu({
        Heading({
            label = caption .. " controls",
            center = true,
            font = "caption",
        }),
        Image({
            texture = image,
            center = true,
            scale = 2,
        }),
        Spacer(32),
        Link({
            label = "Back",
            action = Action.Pop
        }),
        selection_sound = BLIP1,
        pop_sound = BLIP0,
    }))
end

return function()
    sfx.blip(BLIP2)
    return Action.Push(Menu({
        Heading({
            label = "Help",
            center = true,
            font = "caption"
        }),
        Spacer(32),
        Link({
            label = "Ship gamepad",
            action = function() return controls_guide("Ship", "gamepad-guide-ship") end
        }),
        Link({
            label = "Pilot gamepad",
            action = function() return controls_guide("Pilot", "gamepad-guide-pilot") end
        }),
        Link({
            label = "Ship keyboard",
            action = function() return controls_guide("Ship", "keyboard-guide-ship") end
        }),
        Link({
            label = "Pilot keyboard",
            action = function() return controls_guide("Pilot", "keyboard-guide-pilot") end
        }),
        Spacer(32),
        Link({
            label = "Back",
            action = Action.Pop
        }),
        selection_sound = BLIP1,
        pop_sound = BLIP0,
    }))
end
