#![allow(dead_code)]

use std::{collections::{HashMap, BTreeSet}};
use bytecode::lua51::{Proto, Constants, instruction::{Opcode, Instr, Instruction, Reg, RegKst, Kst, UnOp, Opmode, BinCondOp, BinOp}};
use ir::{Context, control_flow::{self, Block}, Op};
use crate::Debug;

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

fn update_register_offsets(closure: &mut Proto, offset: &mut usize, bias: u32) {
	let mut used_registers = BTreeSet::new();

	// assign unique registers
	let mut instr_iter = closure.instructions.iter_mut();
	// ideally all the registers should be in incremental order
	while let Some(instr) = instr_iter.next() {
		// update all registers to be unique to each closure
		match &mut instr.2 {
			Opmode::iABC(a, _b, _c) => {

				match instr.1 {
					Instr::BinCondOp(..) => {}
					_ => {
						if *a >= bias { 
							*a += *offset as u32 - 1;
							used_registers.insert(*a as usize);
							instr.1 = Instr::from_opmode(instr.0, instr.2);
						}
					}
				}

				match &mut instr.1 {
					Instr::NewTable(_, b, c)
					| Instr::Concat(_, b, c) => {
						if b.0 >= bias as u8 { 
							b.0 += *offset as u8 - 1;
							used_registers.insert(b.0 as usize);
						}
						if c.0 >= bias as u8 { 
							c.0 += *offset as u8 - 1;
							used_registers.insert(c.0 as usize);
						}
					}
					Instr::Move(_, b)
					| Instr::LoadNil(_, b) => {
						if b.0 >= bias as u8 { 
							b.0 += *offset as u8 - 1;
							used_registers.insert(b.0 as usize);
						}
					}
					Instr::SetTable(_, b, crk) => {
						if let RegKst::R(rb) = b {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
						if let RegKst::R(rb) = crk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
					}
					Instr::GetTable(_, b, crk)
					| Instr::Self_(_, b, crk) => {
						if b.0 >= bias as u8 { 
							b.0 += *offset as u8 - 1;
							used_registers.insert(b.0 as usize);
						}
						if let RegKst::R(rb) = crk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
					}
					Instr::BinOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
						if let RegKst::R(rb) = crk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
					}
					Instr::UnOp(_, _, b) => {
						if b.0 >= bias as u8 { 
							b.0 += *offset as u8 - 1;
							used_registers.insert(b.0 as usize);
						}
					}
					Instr::BinCondOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
						if let RegKst::R(rb) = crk {
							if rb.0 >= bias as u8 { 
								rb.0 += *offset as u8 - 1;
								used_registers.insert(rb.0 as usize);
							}
						}
					}
					Instr::TestSet(_, b, _) => {
						if b.0 >= bias as u8 { 
							b.0 += *offset as u8 - 1;
							used_registers.insert(b.0 as usize);
						}
					}
					_ => {}
				}

				instr.2 = instr.1.get_opmode();
			}
			Opmode::iABx(a, _bx) => {
				if *a >= bias { 
					*a += *offset as u32 - 1;
					used_registers.insert(*a as usize);
					instr.1 = Instr::from_opmode(instr.0, instr.2);
				}
			}
			Opmode::iAsBx(a, _sbx) => {
				if *a >= bias { 
					*a += *offset as u32 - 1;
					used_registers.insert(*a as usize);
					instr.1 = Instr::from_opmode(instr.0, instr.2);
				}
			},
			Opmode::NOP => {}
		}
	}


	*offset += used_registers.len() // possibly breaks things?
}

pub fn new_register(offset: &mut usize) -> usize {
	*offset += 1;
	*offset
}

pub fn flatten(ctx: &Context, c: &Proto) -> Context {
	let flattened = Proto::default();
	let mut flat_ctx = Context::new(ctx.header, flattened);
	flat_ctx.chunk.nupvals = c.nupvals;
	flat_ctx.chunk.nparams = c.nparams;
	flat_ctx.chunk.source = ctx.chunk.source.clone();
	flat_ctx.chunk.is_vararg_flag = ctx.chunk.is_vararg_flag;
	flat_ctx.chunk.max_stack_size = 250; // fix this later so it's accurate??
	flat_ctx.map();

	let mut debug = Debug::new();

	let mut closure = c.clone();
	flat_ctx.chunk.constants = closure.constants.clone();

	for proto in closure.prototypes.clone() {
		let flattened = flatten(&flat_ctx, &proto);
		flat_ctx.chunk.prototypes.push(flattened.chunk);
	}

	let bias = c.nparams as usize; // if Reg < bias, don't update
	let mut offset = bias; // If Reg > bias, add x to register

	let block_pointer = new_register(&mut offset);
	let block_pointer_r = Reg(block_pointer as u8);
	offset += 3; // 3
	println!("block pointmer register at {block_pointer}");

	update_register_offsets(&mut closure, &mut offset, bias as u32);

	// flatten this closure
	let mut flow = control_flow::Mapper::new();
	let blocks = flow.map(&closure.instructions);

	println!("INSTRUCTIONS BLOCKS: {:?}\n", blocks.iter().map(|block| {
		block.code.iter().map(|instr_pt| {
			closure.instructions[*instr_pt].1.clone()
		}).collect::<Vec<Instr>>()
	}).collect::<Vec<Vec<Instr>>>());

	let mut last_block = 0;

	// generate the leveled control flow if statement
	let mut block_iter = blocks.iter().enumerate().peekable();
	while let Some((i, block)) = block_iter.next() {
		let i = i as i32; // should never be negative
		last_block = i;
		debug.create_block(i as usize);

		let ip_c = flat_ctx.get_or_add_constant(Constants::Number(i as f64));

		let next_target = i as f64 + 1f64;
		let mut target_block = next_target;
		let mut add_target = true;

		debug.if_statement(RegKst::R(block_pointer_r), BinCondOp::Eq, RegKst::K(Kst(256 + ip_c)));

		let mut this = flat_ctx.get_max_ip();
		flat_ctx.add_instruction(this, 
			// ip is located in block_pointer_r, and we're using this instruction pointer as an instruction pointer jump sorta
		 Instruction::new(Opcode::Eq, Instr::BinCondOp(false, RegKst::R(block_pointer_r), BinCondOp::Eq, RegKst::K(Kst(256 + ip_c))))
		);
		this += 1; // may need to make this two or 0
		let d1 = flat_ctx.get_max_ip();

		// bring up block
		let mut iter = block.code.iter();
		while let Some(instr_pt) = iter.next() {
			let inst = closure.instructions.get(*instr_pt).expect("losing instruction");
			let mut clone = inst.clone();
			let mut do_add_instr = true;

			// we are jumping to a new instruction, so get the block that that instruction is in and update properly
			// ex. for loops, jumping
			let b = match inst.1 {
				Instr::Jump(_a, b) => {
					b + 1
				},
				Instr::ForPrep(_a, b) => {
					b + 1
				}
				Instr::ForLoop(_a, b) => {
					b + 1
				}
				Instr::TForLoop(_a, b) => {
					1
				}
				Instr::BinCondOp(..) 
				| Instr::Test(..) => { 1 }
				_ => 0
			};
			if b != 0 {
				println!("--");
				let target = get_block_from_jump(&blocks, *instr_pt, b, i as usize).expect("failed to find jump dest");

				println!("{:?}:: jumping to block, offset: {}. target block: {}, current_block: {}", inst.0, b, target, i);
				match inst.1 {

					// since for loops are different when flattened as they 'jump', lets modify them to fit
					// ps. this was a fucking pain to figure out
					Instr::ForPrep(a, _) => {
						// Stk[A]	= Stk[A] - Stk[A + 2]; -- initial - step
						// InstrPoint	= InstrPoint + Inst[2];

						// b is next target

						clone.0 = Opcode::Sub;
						clone.1 = Instr::BinOp(a, RegKst::R(a), BinOp::Sub, RegKst::R(Reg(a.0 + 2)));
						clone.2 = clone.1.get_opmode();

						// since ForPrep jumps to the ForLoop, we set the ForLoop block as target
						target_block = target as f64;
						debug.for_prep(target);
					}
					Instr::ForLoop(a, _) => {
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Add, Instr::BinOp(a, RegKst::R(a), BinOp::Add, RegKst::R(Reg(a.0 + 2)))));

						// modeled off of rerubi's for loop
						add_target = false;
						do_add_instr = false;

						// Step > 0
						// 0 < step

						// debug.for_loop();

						// 
						let zero = flat_ctx.get_or_add_constant(Constants::Number(0f64));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Lt, Instr::BinCondOp(true, RegKst::R(Reg(a.0 + 2)), BinCondOp::Lt, RegKst::K(Kst(256 + zero)))));
						// jump else statement
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 7))); // 7, was 5?
						// if Index <= Stk[A + 1]
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
						// jump
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 3)));
						//
						let target_pt1 = flat_ctx.get_or_add_constant(Constants::Number(target as f64));
						let target_pt2 = flat_ctx.get_or_add_constant(Constants::Number(next_target));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt1))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 8))); //
						// else
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt2))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 7))); // 

						// possibly need an else statement to set next block to non inner for loop

						// else?
						// if index >= Stk[A + 1]
						// possibly false
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Lt, Instr::BinCondOp(true, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
						// jump
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 3)));
						//
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt1))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 1)));
						// else
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt2))));

						debug.for_loop(target)
					}
					Instr::TForLoop(a, c) => {
						// fuckin misery my guy
						add_target = false;
						do_add_instr = false;

						let r1 = Reg(new_register(&mut offset) as u8);
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::NewTable, 
							Instr::NewTable(r1, block_pointer_r, block_pointer_r)));
						let call = Reg(new_register(&mut offset) as u8);
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, 
							Instr::Move(call, a)));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, 
							Instr::Move(Reg(new_register(&mut offset) as u8), Reg(a.0 + 1))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, 
							Instr::Move(Reg(new_register(&mut offset) as u8), Reg(a.0 + 2))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Call,
							Instr::Call(call, 3, 0)));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::SetList,
							Instr::SetList(r1, 0, 1)));
						

						let r2 = Reg(new_register(&mut offset) as u8);
						let r3 = Reg(new_register(&mut offset) as u8);
						for idx in 1..c + 1 { // possibly 0..?
							let idx_kst = Kst(flat_ctx.get_or_add_constant(Constants::Number(idx as f64)));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK,
								Instr::LoadK(r3, idx_kst)));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetTable,
								Instr::GetTable(r2, r1, RegKst::R(r3))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move,
								Instr::Move(Reg(a.0 + 2 + idx as u8), r2)));
						}
 
						let nil = Kst(flat_ctx.get_or_add_constant(Constants::Nil));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK,
							Instr::LoadK(r2, nil)));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Eq,
							Instr::BinCondOp(true, RegKst::R(Reg(a.0 + 3)), BinCondOp::Eq, RegKst::R(r2))));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump,
							Instr::Jump(block_pointer_r, 3))); // ?????
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move,
							Instr::Move(Reg(a.0 + 2), Reg(a.0 + 3)))); 
						
						// there should be a jump in the next block; lets remove the block
						if let Some((_, next_block)) = block_iter.peek_mut() {
							if let Some(jump_pt) = next_block.code.first() {
								let jump = closure.instructions.get(*jump_pt).expect("losing instruction");
								if let Instr::Jump(_a, b) = jump.1 {
									block_iter.next();
									let inner_for_loop = get_block_from_jump(&blocks, *instr_pt + 1, b + 1, i as usize).expect("jump err");
									let target_pt = flat_ctx.get_or_add_constant(Constants::Number(inner_for_loop as f64));
									flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt))));
									debug.goto_block(i as usize, inner_for_loop as usize, false);
								} else { panic!() }
							} else { panic!() }
						} else { panic!() }
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump,
							Instr::Jump(block_pointer_r, 1)));

						// else
						let target_pt = flat_ctx.get_or_add_constant(Constants::Number(next_target + 1 as f64));
						flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt))));
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
									// block_iter.next();
									println!("oopity doopity");

									flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(inst.0, inst.1.clone()));
									let kst1 = Kst(flat_ctx.get_or_add_constant(Constants::Number(next_target + 1f64)));
									flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
									Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 2)));
									flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, kst1)));
									flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 1)));
									// println!("if true then block {:?} from {}", next_target + 1f64, i);

									// skip next target
									// println!("possibly if statement fucky wucky");
									// println!(" if false then block {} from {}", target, i);
									// what if we don't skip at all?
									let kst2 = Kst(flat_ctx.get_or_add_constant(Constants::Number(target as f64 + 0f64))); // posibly +2?
									flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, kst2)));

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

			if do_add_instr {
				flat_ctx.add_instruction(flat_ctx.get_max_ip(), clone); // @todo: make to remove in the future
			}
		}

		// add jump for conditional
		let dt = flat_ctx.get_max_ip();
		let del = {
			if add_target {
				dt - d1 + 1
			} else {
				dt - d1
			}
		};
		flat_ctx.add_instruction(this, 
		 Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, del as i32))
		);

		// next target
		if add_target {
			let target_pt = flat_ctx.get_or_add_constant(Constants::Number(target_block));
			flat_ctx.add_instruction(flat_ctx.get_max_ip() - 0, Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(target_pt))));
			debug.goto_block(i as usize, target_block as usize, false);
			println!("adding target to {}", target_block);
		}
	}


	let aaa = flat_ctx.get_or_add_constant(Constants::Number(last_block as f64 + 1f64));
	let aaaa = flat_ctx.get_or_add_constant(Constants::Number(-1f64));
	debug.if_statement(RegKst::R(block_pointer_r), BinCondOp::Eq, RegKst::K(Kst(256 + aaa)));
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::Eq, Instr::BinCondOp(false, RegKst::R(block_pointer_r), BinCondOp::Eq, RegKst::K(Kst(256 + aaa))))
	);
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, 1))
	);
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
	 Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(aaaa)))
	);

	// println!("\nINSTRUCTIONS: {:?}", &flat_ctx.chunk.instructions.iter().map(|instr| &instr.1).collect::<Vec<&Instr>>());
	debug.view(&flat_ctx.chunk.constants);

	let entry = flat_ctx.get_or_add_constant(Constants::Number(0f64));

	flat_ctx.add_instruction(0, // load the entry / current point
	Instruction::new(Opcode::LoadK, Instr::LoadK(block_pointer_r, Kst(entry)))
	);

	let binop_pt = flat_ctx.add_instruction(1, // GE than 0
	Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::K(Kst(256 + entry)), BinCondOp::Le, RegKst::R(block_pointer_r)))
	);

	let mut this = flat_ctx.get_max_ip();
	flat_ctx.add_instruction(2, 
	Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, this as i32 - 2)) // delta to ForLoop jump (including itself), subtracts the 2 instructions at top
	);


	let instr_pt = flat_ctx.find_instruction_pt(binop_pt).unwrap() as i32;
	this = flat_ctx.get_max_ip();

	flat_ctx.add_instruction(this, 
		Instruction::new(Opcode::Jump, Instr::Jump(block_pointer_r, instr_pt - this as i32 - 1)) // loop jump; delta to BinOp (including itself)
	);
	flat_ctx.add_instruction(flat_ctx.get_max_ip(), 
		Instruction::new(Opcode::Return, Instr::Return(block_pointer_r, 1))
	);

	//
	flat_ctx
}

pub struct Flatten {
	flat_ctx: Context,
	ctx: Context,
	pointers: HashMap<String, usize>
}

impl Flatten {
	pub fn new(ctx: Context) -> Self {
		let flattened = Proto::default();
		let mut flat_ctx = Context::new(ctx.header, flattened);
		flat_ctx.chunk.source = ctx.chunk.source.clone();
		flat_ctx.chunk.max_stack_size = 250; // fix this later so it's accurate??
		flat_ctx.map();
		
		Self {
			ctx,
			flat_ctx,
			pointers: HashMap::new()
		}
	}

	pub fn flatten(&mut self) {
		let ctx = &mut self.flat_ctx;
		self.flat_ctx = flatten(ctx, &self.ctx.chunk);
		println!("{:?}", self.flat_ctx.chunk);
	}

	pub fn get(self) -> Context {
		self.flat_ctx
	}
}