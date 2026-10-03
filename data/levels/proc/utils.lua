local Utils = {}

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

return Utils