local maths = {}

function maths.angle_difference(a1, a2)
	local d = a2 - a1
	if d > 180 then
		return -360 + d
	elseif d < -180 then
		return 360 + d
	else
		return d
	end
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
