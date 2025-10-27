Scheduler = {}
function Scheduler:new()
    scheduler = {}
    setmetatable(scheduler, self)
    self.__index = self
    return scheduler
end

function Scheduler:add(timeout, callback)
    table.insert(self, {t=timeout, c=callback})
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
function object_scheduler_add(obj, timeout, callback)
    if obj.state.scheduler == nil then
        obj.state.scheduler = Scheduler:new()
    end

    obj.state.scheduler:add(timeout, callback)

    if obj.timer == nil or obj.timer > timeout then
        obj.timer = timeout
    end
end

-- Game object timer callback function
function luola_on_object_timer(obj, timestep)
    return obj.state.scheduler:service(obj, timestep)
end

-- Game global scheduler instance and callback
global_scheduler = Scheduler:new()
function luola_on_global_timer(timestep)
    return global_scheduler:service(nil, timestep)
end

-- Add a callback to the game global scheduler
function global_scheduler_add(timeout, callback)
    global_scheduler:add(timeout, callback)
    game.set_global_timer(0)
end