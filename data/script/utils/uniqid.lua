local UniqID = {
	LAST_ID = 0,
}

function UniqID.new()
	local id = UniqID.LAST_ID + 1
	UniqID.LAST_ID = id
	return id
end

return UniqID
