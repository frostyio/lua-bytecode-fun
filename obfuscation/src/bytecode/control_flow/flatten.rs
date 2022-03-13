use bytecode::lua51::{Proto, Constants, instruction::{Opcode, Instr, Instruction, Reg, RegKst, Kst, Opmode, BinCondOp, BinOp}};
use ir::{Context, control_flow::{self, Block}};
use super::registers::Registers;
use crate::{Debug, bytecode::Options};

// get the block where an instruction pointer is located 
pub fn get_block_from_jump(blocks: &Vec<Block>, ip: usize, offset: i32, current_block: usize) -> Option<usize> {
	let destination = ip as i32 + offset;
	if destination < 0 { panic!("invalid pointer from jump") }

	let dest = destination as usize;
	let possible_blocks = if dest < ip { // jumping backward
		blocks.iter().take(current_block).collect::<Vec<&Block>>()
	} else { // jumping forward
		//blocks.iter().skip(current_block + 1).collect::<Vec<&Block>>()
		blocks.iter().collect::<Vec<&Block>>() // @todo: make it skip but with correct pos 
	};

	possible_blocks.iter().position(|block| {
		if let Some(pos) = block.code.iter().position(|ip| *ip == dest) {
			// println!("destination: {}, possible: {}, ip: {}, offset: {}", dest, pos, ip, offset);
			if pos != 0 {
				panic!("jump does not jump to block start, possible mismatch: {} {} {}", ip, offset, current_block);
			}
			true
		} else {
			false
		}
	})
}

pub fn update_register_offsets(closure: &mut Proto, registers: &mut Registers) {
	let mut instr_iter = closure.instructions.iter_mut();
	while let Some(instr) = instr_iter.next() {
		// update all registers to be unique to each closure
		match &mut instr.2 {
			Opmode::iABC(a, _b, _c) => {

				match instr.1 {
					Instr::BinCondOp(..) => {}
					_ => {
						*a = registers.get(*a as usize).try_into().expect("");
						instr.1 = Instr::from_opmode(instr.0, instr.2);
					}
				}

				match &mut instr.1 {
					Instr::NewTable(_, b, c)
					| Instr::Concat(_, b, c) => {
						b.0 = registers.get(b.0 as usize).try_into().expect("");
						c.0 = registers.get(c.0 as usize).try_into().expect("");
					}
					Instr::Move(_, b)
					| Instr::LoadNil(_, b) => {
						b.0 = registers.get(b.0 as usize).try_into().expect("");
					}
					Instr::SetTable(_, b, crk) => {
						if let RegKst::R(rb) = b {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
						if let RegKst::R(rb) = crk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
					}
					Instr::GetTable(_, b, crk)
					| Instr::Self_(_, b, crk) => {
						b.0 = registers.get(b.0 as usize).try_into().expect("");
						if let RegKst::R(rb) = crk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
					}
					Instr::BinOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
						if let RegKst::R(rb) = crk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
					}
					Instr::UnOp(_, _, b) => {
						b.0 = registers.get(b.0 as usize).try_into().expect("");
					}
					Instr::BinCondOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
						if let RegKst::R(rb) = crk {
							rb.0 = registers.get(rb.0 as usize).try_into().expect("");
						}
					}
					Instr::TestSet(_, b, _) => {
						b.0 = registers.get(b.0 as usize).try_into().expect("");
					}
					_ => {}
				}

				instr.2 = instr.1.get_opmode();
			}
			Opmode::iABx(a, _bx) => {
				*a = registers.get(*a as usize).try_into().expect("");
				instr.1 = Instr::from_opmode(instr.0, instr.2);
			}
			Opmode::iAsBx(a, _sbx) => {
				*a = registers.get(*a as usize).try_into().expect("");
				instr.1 = Instr::from_opmode(instr.0, instr.2);
			},
			Opmode::NOP => {}
		}
	}
}

pub fn flatten(ctx: &Context, c: &Proto, options: &Options) -> Context {
	let target_vm = options.target_vm.get();
	let max_stack_size = target_vm.max_stack_size;
	let mut registers = Registers::from_size(max_stack_size);

	// new closure
	let mut closure = c.clone();

	// create new flattened closure so we can add new instructions to it
	let mut flattened = Proto::default();
	flattened.max_stack_size = max_stack_size as u8; // to update
	flattened.constants = closure.constants.clone();
	flattened.nupvals = closure.nupvals;
	flattened.nparams = closure.nparams;
	flattened.is_vararg_flag = closure.is_vararg_flag;
	flattened.source = closure.source.clone();

	// update registers
	let register_bias = flattened.nparams; // don't use any parameter registers nor update them
	let offset = register_bias; // shift all registers by this offset if less than bias

	(0..register_bias).for_each(|idx| registers.set(idx as usize, idx.into())); // don't change param registers
	registers.offset(offset.into());

	// new context
	let mut flat_ctx = Context::new(ctx.header, flattened);
	flat_ctx.map(); // is this even needed

	let mut debug = Debug::new();

	for proto in &closure.prototypes {
		let flattened = flatten(&flat_ctx, &proto, options);
		flat_ctx.chunk.prototypes.push(flattened.chunk);
	}

	let state_idx = registers.new();
	let state_reg = Reg(state_idx .try_into().expect("unable to make register"));

	// update registers accordingly
	update_register_offsets(&mut closure, &mut registers);

	// flatten this closure
	let mut flow = control_flow::Mapper::new();
	let blocks = flow.map(&closure.instructions);

	// and now we reconstruct the control flow
	let mut flat_blocks: Vec<(i32, Vec<Instruction>)> = vec![]; // block index, block
	let mut last_block = 0;

	let mut block_iter = blocks.iter().enumerate().peekable();
	while let Some((i, block)) = block_iter.next() {
		let i = i as i32;
		last_block = i;

		// automatically add the next target
		let next_target = i as f64 + 1f64;
		let mut target_block = next_target;
		let mut add_target = true;

		let mut instructions: Vec<Instruction> = vec![];
		let mut add = |instruction| instructions.push(instruction);

		// go through all instructions
		let mut iter = block.code.iter();
		while let Some(instr_pt) = iter.next() {
			let inst = closure.instructions.get(*instr_pt).expect("losing instruction");
			let mut clone = inst.clone();
			let mut do_add_instr = true;

			// any instructions we may need to change to make compatible with flattening
			let b_offset = match inst.1 {
				Instr::Jump(_, b)
				| Instr::ForPrep(_, b)
				| Instr::ForLoop(_, b) => b + 1,
				Instr::TForLoop(..) | Instr::BinCondOp(..) | Instr::Test(..) => 1,
				_ => 0
			};

			if b_offset != 0 {
				let target = get_block_from_jump(&blocks, *instr_pt, b_offset, i as usize).expect("failed to find jump dest");

				match inst.1 {
					// since for loops are different when flattened as they 'jump', lets modify them to fit
					// ps. this was a fucking pain to figure out
					Instr::ForPrep(a, _) => {
						// Stk[A]	= Stk[A] - Stk[A + 2]; -- initial - step
						// InstrPoint	= InstrPoint + Inst[2];

						clone.0 = Opcode::Sub;
						clone.1 = Instr::BinOp(a, RegKst::R(a), BinOp::Sub, RegKst::R(Reg(a.0 + 2)));
						clone.2 = clone.1.get_opmode();

						// since ForPrep jumps to the ForLoop, we set the ForLoop block as target
						target_block = target as f64;
					}
					Instr::ForLoop(a, _) => {
						add(Instruction::new(Opcode::Add, Instr::BinOp(a, RegKst::R(a), BinOp::Add, RegKst::R(Reg(a.0 + 2)))));

						// modeled off of rerubi's for loop
						add_target = false;
						do_add_instr = false;

						// 
						let zero = flat_ctx.get_or_add_constant(Constants::Number(0f64));
						add(Instruction::new(Opcode::Lt, Instr::BinCondOp(true, RegKst::R(Reg(a.0 + 2)), BinCondOp::Lt, RegKst::K(Kst(256 + zero)))));
						// jump else statement
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 7)));
						// if Index <= Stk[A + 1]
						add(Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
						// jump
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 3)));
						//
						let target_pt1 = flat_ctx.get_or_add_constant(Constants::Number(target as f64));
						let target_pt2 = flat_ctx.get_or_add_constant(Constants::Number(next_target));
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt1))));
						add(Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 8))); //
						// else
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt2))));
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 7))); // 

						// else?
						// if index >= Stk[A + 1]
						// possibly false
						add(Instruction::new(Opcode::Lt, Instr::BinCondOp(true, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
						// jump
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 3)));
						//
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt1))));
						add(Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 1)));
						// else
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt2))));

						debug.for_loop(target)
					}
					Instr::TForLoop(a, c) => {
						// fuckin misery my guy
						add_target = false;
						do_add_instr = false;

						let r1 = Reg(registers.new() as u8);
						add(Instruction::new(Opcode::NewTable, Instr::NewTable(r1, state_reg, state_reg)));
						let call = Reg(registers.new() as u8);
						add(Instruction::new(Opcode::Move, Instr::Move(call, a)));
						add(Instruction::new(Opcode::Move, Instr::Move(Reg(registers.new() as u8), Reg(a.0 + 1))));
						add(Instruction::new(Opcode::Move, Instr::Move(Reg(registers.new() as u8), Reg(a.0 + 2))));
						add(Instruction::new(Opcode::Call, Instr::Call(call, 3, 0)));
						add(Instruction::new(Opcode::SetList, Instr::SetList(r1, 0, 1)));
						
						let r2 = Reg(registers.new() as u8);
						let r3 = Reg(registers.new() as u8);
						for idx in 1..c + 1 {
							let idx_kst = Kst(flat_ctx.get_or_add_constant(Constants::Number(idx as f64)));
							add(Instruction::new(Opcode::LoadK, Instr::LoadK(r3, idx_kst)));
							add(Instruction::new(Opcode::GetTable, Instr::GetTable(r2, r1, RegKst::R(r3))));
							add(Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 2 + idx as u8), r2)));
						}
 
						let nil = Kst(flat_ctx.get_or_add_constant(Constants::Nil));
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(r2, nil)));
						add(Instruction::new(Opcode::Eq, Instr::BinCondOp(true, RegKst::R(Reg(a.0 + 3)), BinCondOp::Eq, RegKst::R(r2))));
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 3)));
						add(Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 2), Reg(a.0 + 3)))); 
						
						// there should be a jump in the next block; lets remove the block
						if let Some((_, next_block)) = block_iter.peek_mut() {
							if let Some(jump_pt) = next_block.code.first() {
								let jump = closure.instructions.get(*jump_pt).expect("losing instruction");
								if let Instr::Jump(_a, b) = jump.1 {
									block_iter.next();
									let inner_for_loop = get_block_from_jump(&blocks, *instr_pt + 1, b + 1, i as usize).expect("jump err");
									let target_pt = flat_ctx.get_or_add_constant(Constants::Number(inner_for_loop as f64));
									add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt))));
								} else { panic!() }
							} else { panic!() }
						} else { panic!() }
						add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 1)));

						// else
						let target_pt = flat_ctx.get_or_add_constant(Constants::Number(next_target + 1 as f64));
						add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt))));
					}
				
					// any other shit that increases the instruction pointer smh
					Instr::BinCondOp(..)
					| Instr::Test(..) => {
						add_target = false;
						do_add_instr = false;

						// there should be a jump in the next block; lets remove the block
						if let Some((_, next_block)) = block_iter.peek_mut() {
							if let Some(jump_pt) = next_block.code.first() {
								let jump = closure.instructions.get(*jump_pt).expect("losing instruction");
								if let Instr::Jump(_, _) = jump.1 {
									add(Instruction::new(inst.0, inst.1.clone()));
									let kst1 = Kst(flat_ctx.get_or_add_constant(Constants::Number(next_target + 1f64)));
									add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 2)));
									add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, kst1)));
									add(Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 1)));

									// skip next target
									// println!(" if false then block {} from {}", target, i);
									// what if we don't skip at all?
									let kst2 = Kst(flat_ctx.get_or_add_constant(Constants::Number(target as f64)));
									add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, kst2)));
								} else { panic!() }
							} else { panic!() }
						} else { panic!() }
					}
				
					Instr::Jump(..) => {
						do_add_instr = false;
						target_block = target as f64;
					}
					
					_ => {}
				}
			}

			// add instruction if needed
			if do_add_instr {
				add(clone);
			}
		}
	
		if add_target {
			let target_pt = flat_ctx.get_or_add_constant(Constants::Number(target_block));
			add(Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(target_pt))));
		}

		flat_blocks.push((i, instructions));
	}

	// finalizing
	finalize_closure(&mut flat_ctx, state_reg, last_block, flat_blocks);


	flat_ctx
}

fn finalize_closure(flat_ctx: &mut Context, state_reg: Reg, last_block: i32, blocks: Vec<(i32, Vec<Instruction>)>) {
	println!("last block = {last_block}, predicted last block = {}", blocks.len() - 1);

	// add all blocks to chunk instructions & form into possible states
	blocks.into_iter().for_each(|(block_pt, block)| {
		let block_pt_kst = flat_ctx.get_or_add_constant(Constants::Number(block_pt as f64));
		// if statement
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
		 Instruction::new(Opcode::Eq, Instr::BinCondOp(false, RegKst::R(state_reg), BinCondOp::Eq, RegKst::K(Kst(256 + block_pt_kst))))
		);
		// corresponding jump
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
		Instruction::new(Opcode::Jump, Instr::Jump(state_reg, block.len() as i32)) // possibly +1?
	   );

	   block.into_iter().for_each(|instr| { flat_ctx.add_instruction(flat_ctx.get_max_ip(), instr); })
	});

	// block to end the state machine
	let next_block = flat_ctx.get_or_add_constant(Constants::Number(last_block as f64 + 1f64));
	let ending_target = flat_ctx.get_or_add_constant(Constants::Number(-1f64));

	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::Eq, Instr::BinCondOp(false, RegKst::R(state_reg), BinCondOp::Eq, RegKst::K(Kst(256 + next_block))))
	);
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::Jump, Instr::Jump(state_reg, 1))
	);
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(ending_target)))
	);

	// creation of state machine
	let entry = flat_ctx.get_or_add_constant(Constants::Number(0f64));
	flat_ctx.add_instruction(0, // load the entry / current point
	Instruction::new(Opcode::LoadK, Instr::LoadK(state_reg, Kst(entry)))
	);

	let binop_pt = flat_ctx.add_instruction(1, // GE than 0
	Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::K(Kst(256 + entry)), BinCondOp::Le, RegKst::R(state_reg)))
	);

	let mut this = flat_ctx.get_max_ip();
	// if while loop is false, skip
	flat_ctx.add_instruction(2, 
	Instruction::new(Opcode::Jump, Instr::Jump(state_reg, this as i32 - 2)) // delta to ForLoop jump (including itself), subtracts the 2 instructions at top
	);

	let instr_pt = flat_ctx.find_instruction_pt(binop_pt).unwrap() as i32;
	this = flat_ctx.get_max_ip();

	// reset to up
	flat_ctx.add_instruction(this, 
		Instruction::new(Opcode::Jump, Instr::Jump(state_reg, instr_pt - this as i32 - 1)) // loop jump; delta to BinOp (including itself)
	);

	// end instruction
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
		Instruction::new(Opcode::Return, Instr::Return(state_reg, 1))
	);
}