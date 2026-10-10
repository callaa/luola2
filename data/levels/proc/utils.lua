local Utils = {}

function Utils.is_solid(ter)
	return (ter & 0x1f) ~= 0
end

function Utils.is_space(ter)
	return (ter & 0x9f) == 0
end

function Utils.is_water(ter)
	return ter & 0xbf == 0x80
end

-- Scan for solid ground with the given Y axis increment until at most max pixels
-- Returns nil if no solid ground found
function Utils.find_solid_ground(point, d, max)
	for _ = 0,max do
		point = point + d
		if Utils.is_solid(level:terrain_at(point)) then
			return point
		end
	end
	return nil
end

-- Scan for open space (not water) with the given Y axis increment until at most max pixels
-- Returns nil if no space found
function Utils.find_space(point, d, max)
	local t0 = level:terrain_at(point)
	for _ = 0,max do
		point = point + d
		local t = level:terrain_at(point)
		if t ~= t0 then
			if Utils.is_space(t) then
				return point
			else
				return nil
			end
		end
	end
	return nil
end

-- Scan for solid ground or water with the given delta until at most max pixels
-- Returns nil if no non-space found
function Utils.find_non_space(point, d, max)
	for _ = 0,max do
		point = point + d
		if not Utils.is_space(level:terrain_at(point)) then
			return point
		end
	end
	return nil
end

-- Sample pixels around the given point
function Utils.check_all_solid_terrain(point, r)
	if not Utils.is_solid(level:terrain_at(point)) then
		return false
	end
	for x = -3, 3 do
		for y = -3, 3 do
			if not Utils.is_solid(level:terrain_at(point + Vec2(x * r, y * r))) then
				return false
			end
		end
	end
	return true
end

local function bucket_random(bucket)
	return bucket[level:randi(1, #bucket)]
end

-- Split a list of ImageParts into a set of buckets
-- following the "bucketname_xxxx" naming scheme.
-- Returns a {[bucketname] = { ImagePart, ... }, ...} table.
-- The helper function buckets[name]:rand() will return a random ImagePart from the bucket.
function Utils.partbuckets(imageparts)
	local buckets = {}
	for name, img in pairs(imageparts) do
		local sep = string.find(name, "_")
		if sep then
			local bucket_name = string.sub(name, 1, sep - 1)
			if buckets[bucket_name] == nil then
				buckets[bucket_name] = {
					img,
					rand = bucket_random,
				}
			else
				table.insert(buckets[bucket_name], img)
			end
		else
			print("Warning: Image part", name, "not in bucket naming scheme!")
		end
	end
	return buckets
end

-- Remove and return a random item from the list
-- Returns the index and the item
function Utils.take_random_item(items)
	if #items < 1 then
		return nil
	end
	local i = level:randi(1, #items)
	local item = items[i]
	table.remove(items, i)
	return i, item
end

local function square(v) return v*v end

-- Remove a random point, taking in account a list of { point=Vec2, spacing=float } used points
-- The removed point is added to used_points
function Utils.take_random_point_with_spacing(points, used_points, spacing)
	local work = { table.unpack(points) }

	while #work > 0 do
		local i, point = Utils.take_random_item(work)

		local ok = true
		for _, used in ipairs(used_points) do
			if used.point:dist_squared(point) < square(used.spacing + spacing) then
				ok = false
				break
			end
		end

		if ok then
			table.remove(points, i)
			table.insert(used_points, {point=point, spacing=spacing})
			return point
		end
	end
	return nil
end

-- Remove all points that are less than "dist" away from the given point
function Utils.remove_nearby_points(points, near, dist)
	local i = #points
	dist = dist * dist
	while i > 0 do
		if points[i]:dist_squared(near) < dist then
			--level:put_image(points[i].x, points[i].y, debugparts.red_cross, "center")
			table.remove(points, i)
		end
		i = i -1
	end
end

-- Treat the given list of Vec2s as a set of nodes in a fully connected graph
-- and construct a spanning tree by connecting the shortests paths first
-- (Prim's algorithm with euclidian distance as weight)
function Utils.spanning_tree(points)
	local fringe_nodes = { table.unpack(points) }
	local connected_nodes = { table.remove(fringe_nodes) }
	local edges = {}

	while #fringe_nodes > 0 do
		local shortest_f = 0
		local shortest_c = 0
		local shortest_d = 9999 * 9999

		for i, fringe in ipairs(fringe_nodes) do
			for j, connected in ipairs(connected_nodes) do
				local dist = connected:dist_squared(fringe)
				if dist < shortest_d then
					shortest_f = i
					shortest_c = j
					shortest_d = dist
				end
			end
		end

		local f = table.remove(fringe_nodes, shortest_f)
		table.insert(connected_nodes, f)
		table.insert(edges, { connected_nodes[shortest_c], f })
	end

	return edges
end

function Utils.find_dead_ends(edges)
	local vertices = {}
	local function add_vertex(v)
		local name = math.floor(v.x) .. math.floor(v.y)
		
		if vertices[name] == nil then
			vertices[name] = { count = 1, pos = v }
		else
			vertices[name].count = vertices[name].count + 1
		end
	end
	for _, e in ipairs(edges) do
		add_vertex(e[1])
		add_vertex(e[2])
	end

	local dead_ends = {}
	for _, v in pairs(vertices) do
		if v.count < 2 then
			table.insert(dead_ends, v.pos)
		end
	end
	return dead_ends
end

return Utils
