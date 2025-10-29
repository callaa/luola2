local trig = {}

function trig.angle_difference(a1, a2)
	local d = a2 - a1
	if d > 180 then
		return -360 + d
	elseif d < -180 then
		return 360 + d
	else
		return d
	end
end

return trig
