for Idx = 1, n do -- Loading instructions to the chunk.
		-- 	local Data	= gBits32();
		-- 	local Opco	= gBit(Data, 1, 6);
		-- 	local Type	= Opcode[Opco + 1];
		-- 	local Mode  = Opmode[Opco + 1];

		-- 	local Inst	= {
		-- 		Enum	= Opco;
		-- 		Value	= Data;
		-- 		gBit(Data, 7, 14); -- Register A.
		-- 	};

		-- 	if (Type == 'ABC') then -- Most common, basic instruction type.
		-- 		Inst[2]	= gBit(Data, 24, 32);
		-- 		Inst[3]	= gBit(Data, 15, 23);
		-- 	elseif (Type == 'ABx') then
		-- 		Inst[2]	= gBit(Data, 15, 32);
		-- 	elseif (Type == 'AsBx') then
		-- 		Inst[2]	= gBit(Data, 15, 32) - 131071;
		-- 	end;

		-- 	-- Precompute data for some instructions
		-- 	do 
		-- 		-- TEST and TESTSET 
		-- 		if Opco == 26 or Opco == 27 then 
		-- 			Inst[3] = Inst[3] == 0;
		-- 		end

		-- 		-- EQ, LT, LE
		-- 		if Opco >= 23 and Opco <= 25 then 
		-- 			Inst[1] = Inst[1] ~= 0;
		-- 		end 

		-- 		-- Anything that looks at a constant using B
		-- 		if Mode.b == 'OpArgK' then
		-- 			Inst[3] = Inst[3] or false; -- Simply to guarantee that Inst[4] is inserted in the array part
		-- 			if Inst[2] >= 256 then 
		-- 				local Cons = Inst[2] - 256;
		-- 				Inst[4] = Cons;

		-- 				local ReferenceData = ConstantReferences[Cons];
		-- 				if not ReferenceData then 
		-- 					ReferenceData = {};
		-- 					ConstantReferences[Cons] = ReferenceData;
		-- 				end

		-- 				ReferenceData[#ReferenceData + 1] = {Inst = Inst, Register = 4}
		-- 			end
		-- 		end 

		-- 		-- Anything that looks at a constant using C
		-- 		if Mode.c == 'OpArgK' then
		-- 			Inst[4] = Inst[4] or false -- Simply to guarantee that Inst[5] is inserted in the array part
		-- 			if Inst[3] >= 256 then 
		-- 				local Cons = Inst[3] - 256;
		-- 				Inst[5] = Cons;

		-- 				local ReferenceData = ConstantReferences[Cons];
		-- 				if not ReferenceData then 
		-- 					ReferenceData = {};
		-- 					ConstantReferences[Cons] = ReferenceData;
		-- 				end

		-- 				ReferenceData[#ReferenceData + 1] = {Inst = Inst, Register = 5}
		-- 			end
		-- 		end 
		-- 	end

		-- 	Instr[Idx]	= Inst;
		-- end;