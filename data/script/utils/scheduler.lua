-- Scheduler class that allows multiple indepedent scheduled callbacks
-- to be added to objects with just a single timer.

local Scheduler = {}
function Scheduler:new()
	local scheduler = {}
	setmetatable(scheduler, self)
	self.__index = self
	return scheduler
end

function Scheduler:add(timeout, callback)
	table.insert(self, { t = timeout, c = callback })
	return self
end

function Scheduler:service(context, timestep)
	local next_timer = nil
	for i = #self, 1, -1 do
		local timer = self[i]
		local t = timer.t - timestep
		if t <= 0 then
			local rerun = timer.c(context)
			if rerun ~= nil then
				timer.t = rerun
				if next_timer == nil or rerun < next_timer then
					next_timer = rerun
				end
			else
				table.remove(self, i)
			end
		else
			timer.t = t
			if next_timer == nil or t < next_timer then
				next_timer = t
			end
		end
	end
	return next_timer
end

-- Add a scheduled callback to an object that has a state property and a timer
function Scheduler.add_to_object(obj, timeout, callback)
	if obj.state.scheduler == nil then
		obj.state.scheduler = Scheduler:new()
	end

	obj.state.scheduler:add(timeout, callback)

	if obj.timer == nil or obj.timer > timeout then
		obj.timer = timeout
	end
end

-- Game global scheduler instance
Scheduler._global = Scheduler:new()

-- Add a global scheduled callback
function Scheduler.add_global(timeout, callback)
	Scheduler._global:add(timeout, callback)
	game.set_global_timer(0)
end

return Scheduler
