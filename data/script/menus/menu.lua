function main_menu()
	return Menu({
		Image({
			texture = "gamelogo",
			center = true,
		}),
		Spacer(32),
		Link({
			label = "Start!",
			action = function() return Action.Return("start") end,
		}),
		Link({
			label = "Settings",
			action = settings_menu
		}),
		Link({
			label = "Quit",
			action = function() return Action.Return("quit") end,
		}),
	})
end

function settings_menu()
	SETTINGS = load_settings()
	SETTINGS_CHANGED = false

	return Action.Push(Menu({
		Heading({
			label = "Settings",
			center = true,
			font = "caption",
		}),
		Spacer(32),
		Link({
			label = "Video",
			action = video_menu,
		}),
		Link({
			label = "Game",
			action = game_menu,
		}),
		Link({
			label = "Keyboard",
			action = keyboard_menu,
		}),
		Spacer(16),
		Link({
			label = "Back",
			action = Action.Pop,
		}),
		on_exit = function()
			if SETTINGS_CHANGED then
				save_settings(SETTINGS)
				SETTINGS_CHANGED = false
			end
		end,
	}))
end

function video_menu()
	return Action.Push(Menu({
		Heading({
			label = "Video settings",
			center = true,
			font = "caption",
		}),
		Spacer(32),
		Link({
			label = "Start in fullscreen mode:",
			value = Value.Toggle(SETTINGS.video.fullscreen),
			action = function(item)
				SETTINGS.video.fullscreen = item:toggle()
				SETTINGS_CHANGED = true
			end,
		}),
		Spacer(16),
		Link({
			label = "Back",
			action = Action.Pop,
		}),
	}))
end

function game_menu()
	return Action.Push(Menu({
		Heading({
			label = "Game options",
			center = true,
			font = "caption",
		}),
		Spacer(32),
		Link({
			label = "Show minimap: ",
			value = Value.Toggle(SETTINGS.game.minimap),
			action = function(item)
				SETTINGS.game.minimap = item:toggle()
				SETTINGS_CHANGED = true
			end,
		}),
		Link({
			label = "Rebuild bases: ",
			value = Value.Toggle(SETTINGS.game.baseregen),
			action = function(item)
				SETTINGS.game.baseregen = item:toggle()
				SETTINGS_CHANGED = true
			end,
		}),
		Spacer(16),
		Link({
			label = "Back",
			action = Action.Pop,
		}),
	}))
end

function keyboard_menu()
	return Action.Push(Menu({
		Heading({
			label = "Keyboard controls",
			center = true,
			font = "caption",
		}),
		Spacer(32),
		
		Link({
			label = "Keyboard 1",
			action = function() return keymap_menu(1) end,
		}),
		Link({
			label = "Keyboard 2",
			action = function() return keymap_menu(2) end,
		}),
		Link({
			label = "Keyboard 3",
			action = function() return keymap_menu(3) end,
		}),
		Link({
			label = "Keyboard 4",
			action = function() return keymap_menu(4) end,
		}),
		
		Spacer(16),
		Link({
			label = "Back",
			action = Action.Pop,
		}),
	}))
end

function keymap_menu(id)
	local keymap = SETTINGS["keymap" .. id]
	if not keymap then
		keymap = get_default_keymap(id)
	end

	return Action.Push(Menu({
		Heading({
			label = "Keyboard " .. id,
			center = true,
			font = "caption",
		}),
		Spacer(32),
		
		Link({
			label = "Up: ",
			value = Value.KeyGrab(keymap.thrust),
			action = Action.KeyGrab,
		}),
		Link({
			label = "Down: ",
			value = Value.KeyGrab(keymap.down),
			action = Action.KeyGrab,
		}),
		Link({
			label = "Left: ",
			value = Value.KeyGrab(keymap.left),
			action = Action.KeyGrab,
		}),
		Link({
			label = "Right: ",
			value = Value.KeyGrab(keymap.right),
			action = Action.KeyGrab,
		}),
		Link({
			label = "Fire 1: ",
			value = Value.KeyGrab(keymap.fire_primary),
			action = Action.KeyGrab,
		}),
		Link({
			label = "Fire 2: ",
			value = Value.KeyGrab(keymap.fire_secondary),
			action = Action.KeyGrab,
		}),
		
		Spacer(16),
		Link({
			label = "Back",
			action = Action.Pop,
		}),
		on_exit = function(values)
			keymap.thrust = values[3]
			keymap.down = values[4]
			keymap.left = values[5]
			keymap.right = values[6]
			keymap.fire_primary = values[7]
			keymap.fire_secondary = values[8]
			SETTINGS["keymap" .. id] = keymap
			SETTINGS_CHANGED = true
		end,
	}))
end