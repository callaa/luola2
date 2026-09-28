local help_menu = require("menus.help")

function main_menu()
	return Menu({
		Heading({
			label = "Paused",
			font = "big",
			center = true,
		}),
		Spacer(32),
		Link({
			label = "Resume",
			action = function() return Action.Return("resume") end,
		}),
		Link({
			label = "Help",
			action = help_menu
		}),
		Spacer(16),
		Link({
			label = "End round",
			action = function() return Action.Return("endround") end,
		}),
		Link({
			label = "End game",
			action = function() return Action.Return("endgame") end,
		}),
	})
end
