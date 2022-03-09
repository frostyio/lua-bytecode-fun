local regs = {
	[0] = 0
}

while regs[0] >= 0 do 
	if regs[0] == 0 then
		regs[0] = 1
	end
	if regs[0] == 1 then 
		regs[0] = 2
	end
	if regs[0] == 2 then 
		print("hi")
		regs[1] = 1
		regs[2] = 10
		regs[3] = 1
		regs[1] = regs[1] - regs[3]
		regs[0] = 4 
	end
	if regs[0] == 3 then 
		print(regs[4])
		regs[0] = 4
	end
	if regs[0] == 4 then 
		regs[1] = regs[1] + regs[3]
		if regs[3] > 0 then 
			if regs[1] <= regs[2] then 
				regs[0] = 3
				regs[4] = regs[1]
			else
				regs[0] = 5
			end
		else
			if regs[1] >= regs[2] then 
				regs[0] = 3
				regs[4] = regs[1]
			else
				regs[0] = 5
			end
		end
	end
end
