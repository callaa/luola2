local Level = {}

-- Convert a list of coordinates from level to world coordinates
function Level.to_world_coordinates(coordinates)
	local w = {}
	for _, c in ipairs(coordinates) do
		table.insert(w, c * 3)
	end
	return w
end

return Level
