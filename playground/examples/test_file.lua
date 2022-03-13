local reg = {
	[0] = 0 -- reg 0
}

local q = 20
local function new_reg()
	q = q + 1
	return q
end

while reg[0] >= 0 do
	if reg[0] == 0 then
		reg[1] = {} -- new table
		reg[2] = "a" -- loadk
		reg[3] = "b" -- loadk
		reg[1][1] = reg[2] -- setlist
		reg[1][2] = reg[3] -- "     "
		reg[2] = pairs -- get global
		reg[3] = reg[1] -- move
		reg[2] = reg[2](reg[3]) -- call

		reg[0] = 2 -- progress to tforloop block
	end

	if reg[0] == 1 then
		reg[7] = print -- get global
		reg[8] = reg[6] -- move
		reg[7](reg[8])
		reg[0] = 2
	end

	if reg[0] == 2 then
		-- 2, 3, 4, 5 tforloop registers
		local a = 2 -- links to call
		local c = 2 -- maybe?

		local r1 = new_reg()
		reg[r1] = {}
		reg[new_reg()] = reg[a + 1]
		reg[new_reg()] = reg[a + 2]
		reg[r1] = {reg[a](reg[a + 1], reg[a + 2])} -- set list & call

		-- forprep
		for idx = 1, c do 
			reg[a + 2 + idx] = reg[r1][idx]
		end -- forloop

		if reg[a + 3] ~= nil then
			reg[a + 2] = reg[a + 3]
			reg[0] = 1 -- jump
		else
			reg[0] = 3
		end 
	end

	if reg[0] == 3 then
		reg[0] = -1
	end
end
