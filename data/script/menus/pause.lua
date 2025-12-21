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
			label = "End round",
			action = function() return Action.Return("endround") end,
		}),
		Link({
			label = "End game",
			action = function() return Action.Return("endgame") end,
		}),
	})
end