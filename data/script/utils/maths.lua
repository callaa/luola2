local maths = {}

function maths.angle_difference(a1, a2)
	return (a1 - a2 + 180) % 360 - 180
end

function maths.signum(val)
	if val < 0 then
		return -1
	elseif val > 0 then
		return 1
	end
	return 0
end

return maths
