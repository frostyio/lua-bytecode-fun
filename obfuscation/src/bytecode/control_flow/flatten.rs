#![allow(dead_code)]

use std::{collections::{HashMap, BTreeSet}};

use bytecode::lua51::{Proto, Constants, instruction::{Opcode, Instr, Instruction, Reg, RegKst, Kst, Opmode, BinCondOp, BinOp}};
use ir::{Context, control_flow::{self, Block}, Op};
use crate::Debug;

pub fn update_mode(mode: &mut Opmode, val: Op, func: fn(u32) -> u32)  {
	match mode {
		Opmode::iABC(a, b, c) => {
			match val {
				Op::A => *a = func(*a),
				Op::B => *b = func(*b),
				Op::C => *c = func(*c),
				_ => panic!()
			}
		},
		Opmode::iABx(a, bx) => {
			match val {
				Op::A => *a = func(*a),
				Op::Bx => *bx = func(*bx),
				_ => panic!()
			}
		}
		Opmode::iAsBx(a, sbx) => {
			match val {
				Op::A => *a = func(*a),
				Op::sBx => *sbx = func(*sbx),
				_ => panic!()
			}
		},
		_ => {}
	}
}

pub fn update_register_offsets(closure: &mut Proto, offset: &mut usize) {
	*offset += 1; // increase by one each time 
	let mut used_registers = BTreeSet::new();

	// assign unique registers
	let mut instr_iter = closure.instructions.iter_mut();
	// ideally all the registers should be in incremental order
	while let Some(instr) = instr_iter.next() {
		match &mut instr.1 {
			Instr::Closure(_, b) => {
				let c = &mut closure.prototypes[*b as usize];
				update_register_offsets(c, offset)
			},
			_ => {}
		}

		// update all registers to be unique to each closure
		match &mut instr.2 {
			Opmode::iABC(a, _b, _c) => {
				*a += *offset as u32 - 1;
				used_registers.insert(*a as usize);
				instr.1 = Instr::from_opmode(instr.0, instr.2);

				match &mut instr.1 {
					Instr::NewTable(_, b, c)
					| Instr::Concat(_, b, c) => {
						b.0 += *offset as u8 - 1;
						used_registers.insert(b.0 as usize);
						c.0 += *offset as u8 - 1;
						used_registers.insert(c.0 as usize);
					}
					Instr::Move(_, b)
					| Instr::LoadNil(_, b) => {
						b.0 += *offset as u8 - 1;
						used_registers.insert(b.0 as usize);
					}
					Instr::SetTable(_, b, crk) => {
						if let RegKst::R(rb) = b {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
						if let RegKst::R(rb) = crk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
					}
					Instr::GetTable(_, b, crk) => {
						b.0 += *offset as u8 - 1;
						used_registers.insert(b.0 as usize);
						if let RegKst::R(rb) = crk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
					}
					Instr::BinOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
						if let RegKst::R(rb) = crk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
					}
					Instr::UnOp(_, _, b) => {
						b.0 += *offset as u8 - 1;
						used_registers.insert(b.0 as usize);
					}
					Instr::BinCondOp(_, brk, _, crk) => {
						if let RegKst::R(rb) = brk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
						if let RegKst::R(rb) = crk {
							rb.0 += *offset as u8 - 1;
							used_registers.insert(rb.0 as usize);
						}
					}
					Instr::TestSet(_, b, _) => {
						b.0 += *offset as u8 - 1;
						used_registers.insert(b.0 as usize);
					}
					_ => {}
				}

				instr.2 = instr.1.get_opmode();
			}
			Opmode::iABx(a, _bx) => {
				*a += *offset as u32 - 1;
				used_registers.insert(*a as usize);
				instr.1 = Instr::from_opmode(instr.0, instr.2);
			}
			Opmode::iAsBx(a, _sbx) => {
				*a += *offset as u32 - 1;
				used_registers.insert(*a as usize);
				instr.1 = Instr::from_opmode(instr.0, instr.2);
			},
			Opmode::NOP => {}
		}
	}

	// if the registers aren't in incremental order, this doesn't work
	*offset += used_registers.len();
}

pub fn pre_flatten_closures(c: &Proto, register_offset: &mut usize) -> Proto {
	let mut closure = c.clone();

	update_register_offsets(&mut closure, register_offset);
	
	let mut instr_iter = closure.instructions.iter_mut();
	while let Some(instr) = instr_iter.next() {
		match &mut instr.1 {
			Instr::Closure(_, b) => {
				// b = closure index

				/*
				proto upvals:
				loop through all up values and update corresponding getupval

				ex.
				proto 0:
					// an empty proto
				proto 1:
					GetUpval(Reg(0), 0) // gets the upvalue which is proto 0, anything referencing Reg(0) in this closure is referenced 
										// to the upval's original register (@ point a), which is also Reg(0)
				closure 0:
					Closure(Reg(0), 0)
					Closure(Reg(1), 1)
					Move(Reg(0), Reg(0)) // the upvalue referencing proto 0 (point a)
				
				*/

				let new_closure = &mut closure.prototypes[*b as usize];
				let n_upvals = new_closure.nupvals;
				let mut upval_map: HashMap<u8, u8> = HashMap::new(); // upval number, register

				for n in 0..n_upvals {
					// for every up value that is given to the closure
					if let Some(next_instr) = instr_iter.next() { // e.g. GetUpval or Move always come after a closure if it has params
						match next_instr.1 {
							Instr::Move(a, _) => {
								let upval = a.0; // the register of a soon to be upvalue
								upval_map.insert(n, upval);
								println!("there's an upvalue at register {}", a.0);
							},
							Instr::GetUpval(..) => unimplemented!(),
							_ => {}
						}
					}
				}
			
				// go through the closure's instructions and update the upval references
				for inst in &mut new_closure.instructions {
					match inst.1 {
						Instr::GetUpval(a, b) => {
							if let Some(upval) = upval_map.get(&(b as u8)) {
								// upval is register where the upval is assigned
								inst.0 = Opcode::Move;
								inst.1 = Instr::Move(a, Reg(*upval));
								inst.2 = inst.1.get_opmode();
								println!("replacing upval with flattened value");
							}
						},
						Instr::SetUpval(..) => todo!(),
						_ => {}
					}
				}
			},
			_ => {}
		}
	};

	closure
}

pub fn flatten_closures(flattened: &mut Vec<Proto>, mut closure: Proto) {
	for _ in 0..closure.prototypes.len() {
		let clos = closure.prototypes.remove(0);
		flatten_closures(flattened, clos);
	}

	flattened.push(closure);
}

pub fn get_block_from_jump(blocks: &Vec<Block>, ip: usize, offset: i32, current_block: usize) -> Option<usize> {
	let destination = ip as i32 + offset;
	if destination < 0 {
		panic!("invalid pointer from jump");
	}
	let dest = destination as usize;

	let possible_blocks = if dest < ip { // jumping backward
		blocks.iter().take(current_block).collect::<Vec<&Block>>()
	} else { // jumping forward
		//blocks.iter().skip(current_block + 1).collect::<Vec<&Block>>()
		blocks.iter().collect::<Vec<&Block>>() // @todo: make it skip but with correct pos 
	};

	println!("{}, {}", destination, ip);

	possible_blocks.iter().position(|block| {
		if let Some(pos) = block.code.iter().position(|ip| *ip == dest) {
			println!("destination: {}, possible: {}, ip: {}, offset: {}", dest, pos, ip, offset);
			if pos != 0 {
				println!("jump does not jump to block start, possible mismatch: {} {} {}", ip, offset, current_block);
			}
			true
		} else {
			false
		}
	})
}

pub fn get_closure(closure_id: i32) -> i32 {
	(1 << (closure_id * 5)) - 1
}

pub fn get_new_register(offset: &mut usize) -> u8 {
	*offset += 1;
	*offset as u8
}

pub fn flatten(flat_ctx: &mut Context, c: &Proto) -> u32 {
	let mut debug = Debug::new();

	// function return queue 
	// when a function is called, when it's ready to return to it's call block,
	// it pops the first value off the table and returns to the point of where it was popped
	flat_ctx.add_instruction(0, Instruction::new(Opcode::NewTable, Instr::NewTable(Reg(1), Reg(0), Reg(0))));

	let mut register_offset: usize = 2; // the inital Reg(0) offset + function return queue 
	let closure = pre_flatten_closures(c, &mut register_offset);

	// function block queue registers
	let table_kst = Kst(flat_ctx.get_or_add_constant(Constants::String("table".to_string())));
	let table_kst2 = Kst(flat_ctx.get_or_add_constant(Constants::String("remove".to_string())));
	let table_kst3 = Kst(flat_ctx.get_or_add_constant(Constants::String("insert".to_string())));
	let table_kst4 = Kst(flat_ctx.get_or_add_constant(Constants::Number(1f64)));
	let queue_1 = Reg(get_new_register(&mut register_offset)); // getglobal, gettable :: "table" "insert"
	let queue_2 = Reg(get_new_register(&mut register_offset)); // move :: 2 0
	let queue_3 = Reg(get_new_register(&mut register_offset)); // loadk :: 0

	// let pop = |val: Constants| {
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(queue_1, table_kst)));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetTable, Instr::GetTable(queue_1, queue_1, RegKst::K(table_kst2))));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(queue_2, Reg(1))));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(queue_3, table_kst4)));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Call, Instr::Call(queue_1, 3, 1)));
	// };

	// let push = |val: Kst| {
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(queue_1, table_kst)));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetTable, Instr::GetTable(queue_1, queue_1, RegKst::K(table_kst3))));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(queue_2, Reg(1))));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(queue_3, val)));
	// 	flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Call, Instr::Call(queue_1, 3, 1)));
	// };

	let mut max_block_ip: u32 = 0;

	// flatten all inner closures
	let mut flattened: Vec<Proto> = vec![];
	// 0 is the entry point of every closure/Vec<Block>
	flatten_closures(&mut flattened, closure);

	let closure_id = flattened.len();
	let mut clos_n: i32 = -1;
	for closure_n in (0..closure_id).rev() {
		let mut closure = flattened.remove(closure_n);
		clos_n += 1;

		let this_id = get_closure(clos_n);
		debug.create_closure(this_id as usize);

		println!("\n--CREATING CLOSURE #{}, id: {}", clos_n, this_id);

		// flatten this closure
		let mut flow = control_flow::Mapper::new();
		let blocks = flow.map(&closure.instructions);
		// println!("blocks: {:?}", blocks);

		println!("\nINSTRUCTIONS NON-BLOCKS: {:?}", &closure.instructions.iter().map(|instr| &instr.1).collect::<Vec<&Instr>>());
		println!("INSTRUCTIONS BLOCKS: {:?}\n", blocks.iter().map(|block| {
			block.code.iter().map(|instr_pt| {
				closure.instructions[*instr_pt].1.clone()
			}).collect::<Vec<Instr>>()
		}).collect::<Vec<Vec<Instr>>>());

		// push constants up & remap
		for instr in closure.instructions.iter_mut() {
			match &mut instr.1 {
				Instr::Self_(_, _, c)
				| Instr::GetTable(_, _, c) => {
					match c {
						RegKst::K(kst) => {
							let k = closure.constants[kst.0 as usize - 255].clone();
							kst.0 = flat_ctx.get_or_add_constant(k) as u32;
						}
						_ => {}
					}
				},
				Instr::LoadK(_, b) 
				| Instr::GetGlobal(_, b)
				| Instr::SetGlobal(_, b) => {
					let k = closure.constants[b.0 as usize].clone();
					b.0 = flat_ctx.get_or_add_constant(k) as u32;
				}
				Instr::BinOp(_, a, _, c)
				| Instr::BinCondOp(_, a, _, c) => {
					match a {
						RegKst::K(kst) => {
							let k = closure.constants[kst.0 as usize - 256].clone();
							kst.0 = flat_ctx.get_or_add_constant(k) as u32;
						}
						_ => {}
					}
					match c {
						RegKst::K(kst) => {
							let k = closure.constants[kst.0 as usize - 256].clone();
							kst.0 = flat_ctx.get_or_add_constant(k) as u32;
						}
						_ => {}
					}
				}
				_ => {}
			}
		}

		// closures used by the proto vs global functions
		let mut function_registers: HashMap<u32, usize> = HashMap::new();

		// generate the leveled control flow if statement
		for (i, block) in blocks.iter().enumerate() {
			let i = this_id + (i as i32); // should never be negative
			println!("creating block {}, {} + {}", i, this_id, i);
			debug.create_block(i as usize);

			max_block_ip += 1;
			let ip_c = flat_ctx.get_or_add_constant(Constants::Number(i as f64));

			let next_target = i as f64 + 1f64;
			let mut target_block = next_target;
			let mut add_target = true;

			debug.if_statement(RegKst::R(Reg(0)), BinCondOp::Eq, RegKst::K(Kst(256 + ip_c)));

			let mut this = flat_ctx.get_max_ip();
			flat_ctx.add_instruction(this, 
				// ip is located in Reg(0), and we're using this instruction pointer as an instruction pointer jump sorta
			 Instruction::new(Opcode::Eq, Instr::BinCondOp(false, RegKst::R(Reg(0)), BinCondOp::Eq, RegKst::K(Kst(256 + ip_c))))
			);
			this += 1; // may need to make this two or 0
			let mut code_size = block.code.len() + 1;
			let d1 = flat_ctx.get_max_ip();

			// bring up block
			let mut iter = block.code.iter();
			while let Some(instr_pt) = iter.next() {
				let inst = closure.instructions.get(*instr_pt).expect("losing instruction");
				let mut clone = inst.clone();
				let mut do_add_instr = true;

				// for closure, consume next move/getupvalues for [pt..pt+A] and replace all references that register to 
				// original value
				match inst.1 {
					Instr::Closure(a, b) => {
						let target = get_closure((b + 1) as i32);
						let chunk_kst = flat_ctx.get_or_add_constant(Constants::Number(target as f64));
						clone.0 = Opcode::LoadK;
						clone.1 = Instr::LoadK(a, Kst(chunk_kst));
						clone.2 = clone.1.get_opmode();
						function_registers.insert(a.0 as u32, target as usize);
						println!("setting closure at reg {} pointing at {}", a.0, target);

						// for i in 0..
					}
					Instr::Move(a, b) => {
						if function_registers.contains_key(&(a.0 as u32)) {
							function_registers.remove(&(a.0 as u32));
						}
						let contains = function_registers.contains_key(&(b.0 as u32));
						if contains {
							let value = *function_registers.get(&(b.0 as u32)).unwrap();
							function_registers.insert(a.0 as u32, value);
							println!("m: setting closure at reg {} pointing at {}", a.0, value);
						}
					}
					Instr::SetGlobal(a, _b) => {
						function_registers.remove(&(a.0 as u32));
					}
					Instr::Call(a, _b, _c) => {
						let target_closure = a.0;
						let contains = function_registers.contains_key(&(a.0 as u32));
						println!("looking for closure at reg {}, {}", target_closure, contains);
						if contains {
							println!("using closure function, pointing to {}",target_closure as f64);
							// do_add_instr = false;
							// code_size -= 1;
							// target_block = target_closure as f64;
							add_target = false;
							clone.0 = Opcode::Move;
							clone.1 = Instr::Move(Reg(0), Reg(target_closure));
							clone.2 = clone.1.get_opmode();
							debug.goto_block(i as usize, *function_registers.get(&(a.0 as u32)).unwrap());

							// return block queue
							let kst = Kst(flat_ctx.get_or_add_constant(Constants::Number(next_target)));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(queue_1, table_kst)));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetTable, Instr::GetTable(queue_1, queue_1, RegKst::K(Kst(256 + table_kst3.0)))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(queue_2, Reg(1))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(queue_3, kst)));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Call, Instr::Call(queue_1, 3, 1)));
						} else {
							println!("calling global");
						}
					}
					
					Instr::Return(_a, _b) => {
						// todo
						do_add_instr = false;
						code_size -= 1;
					}
					_ => {}
				}

				// we are jumping to a new instruction, so get the block that that instruction is in and update properly
				// ex. for loops, jumping
				let b = match inst.1 {
					Instr::Jump(_a, b) => {
						b  // + 1?
					},
					Instr::ForPrep(_a, b) => {
						b + 1
					}
					Instr::ForLoop(_a, b) => {
						b + 1
					}
					_ => 0
				};
				if b != 0 {
					println!("--");
					let mut target = get_block_from_jump(&blocks, *instr_pt, b, i as usize - this_id as usize).expect("failed to find jump dest");
					target += this_id as usize;

					println!("{:?}:: jumping to block, offset: {}. target block: {}, current_block: {}", inst.0, b, target, i);
					match inst.1 {
						// since for loops are different when flattened as they 'jump', lets modify them to fit
						// ps. this was a fucking pain to figure out
						Instr::ForPrep(a, b) => {
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
						Instr::ForLoop(a, b) => {
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
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 7))); // 7, was 5?
							// if Index <= Stk[A + 1]
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
							// jump
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 3)));
							//
							let target_pt1 = flat_ctx.get_or_add_constant(Constants::Number(target as f64));
							let target_pt2 = flat_ctx.get_or_add_constant(Constants::Number(next_target));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt1))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 8))); //
							// else
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt2))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 7))); // 

							// possibly need an else statement to set next block to non inner for loop

							// else?
							// if index >= Stk[A + 1]
							// possibly false
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Lt, Instr::BinCondOp(true, RegKst::R(a), BinCondOp::Le, RegKst::R(Reg(a.0 + 1)))));
							// jump
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 3)));
							//
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt1))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(Reg(a.0 + 3), Reg(a.0))));
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), 1)));
							// else
							flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt2))));

							code_size += 12;
							debug.for_loop(target)
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
			let del = dt - d1 + 1;
			flat_ctx.add_instruction(this, 
			 Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), del as i32))
			);

			// next target
			if add_target {
			let target_pt = flat_ctx.get_or_add_constant(Constants::Number(target_block));
				flat_ctx.add_instruction(flat_ctx.get_max_ip() - 0, Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt))));
				debug.goto_block(i as usize, target_block as usize);
				println!("adding target to {}", target_block);
			}
		}

		// end target, proceed to next closure?
		// let target_pt = flat_ctx.get_or_add_constant(Constants::Number(blocks.len() as f64 + 1f64));
		// flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(target_pt))));
		// return block queue

		// return to where
		let kst = Kst(flat_ctx.get_or_add_constant(Constants::Number(next_target)));
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(queue_1, table_kst)));
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::GetTable, Instr::GetTable(queue_1, queue_1, RegKst::K(Kst(256 + table_kst3.0)))));
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Move, Instr::Move(queue_2, Reg(1))));
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::LoadK, Instr::LoadK(queue_3, kst)));
		flat_ctx.add_instruction(flat_ctx.get_max_ip(), Instruction::new(Opcode::Call, Instr::Call(queue_1, 3, 1)));
	}

	// println!("\nINSTRUCTIONS: {:?}", &flat_ctx.chunk.instructions.iter().map(|instr| &instr.1).collect::<Vec<&Instr>>());
	debug.view(&flat_ctx.chunk.constants);

	//
	max_block_ip
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

	pub fn append_end_flatten_statement(&mut self, max_block: u32) {
			/*
		local i = 0
		while i <= X do
		end
		 */

		let ctx = &mut self.flat_ctx;
		let entry = ctx.get_or_add_constant(Constants::Number(0f64));
		// let max = ctx.get_or_add_constant(Constants::Number(max_block as f64)); // self.max_closure_ip.into()));

		ctx.add_instruction(0, // load the entry / current point
		Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(0), Kst(entry)))
		);
		let binop_pt = ctx.add_instruction(1, // GE than 0
		Instruction::new(Opcode::Le, Instr::BinCondOp(false, RegKst::K(Kst(256 + entry)), BinCondOp::Le, RegKst::R(Reg(0))))
		);
		self.pointers.insert("while_binop".to_string(), binop_pt);
 
		let mut this = ctx.get_max_ip();
		ctx.add_instruction(2, 
		Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), this as i32 - 1)) // delta to ForLoop jump (including itself)
		);


		let instr_pt = ctx.find_instruction_pt(binop_pt).unwrap() as i32;
		this = ctx.get_max_ip();

		ctx.add_instruction(this, 
			Instruction::new(Opcode::Jump, Instr::Jump(Reg(0), instr_pt - this as i32 - 1)) // loop jump; delta to BinOp (including itself)
		);
		ctx.add_instruction(ctx.get_max_ip(), 
			Instruction::new(Opcode::Return, Instr::Return(Reg(0), 1))
		);
	}

	pub fn flatten(&mut self) {
		let ctx = &mut self.flat_ctx;
		let max_block = flatten(ctx, &self.ctx.chunk);


		// let print = ctx.get_or_add_constant(Constants::String("print".to_string()));
		// let hello = ctx.get_or_add_constant(Constants::String("SPIKE GAY".to_string()));
		// ctx.add_instruction(0, Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(Reg(1), Kst(print))));
		// ctx.add_instruction(1, Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(2), Kst(hello))));
		// ctx.add_instruction(2, Instruction::new(Opcode::Call, Instr::Call(Reg(1), 2, 1)));
		
		self.append_end_flatten_statement(max_block + 1);
	}

	pub fn get(self) -> Context {
		self.flat_ctx
	}
}