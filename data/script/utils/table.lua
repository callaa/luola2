local tableutils = {}

function tableutils.combined(...)
	local t = {}
	for _, tbl in ipairs({ ... }) do
		for k, v in pairs(tbl) do
			t[k] = v
		end
	end
	return t
end

return tableutils
