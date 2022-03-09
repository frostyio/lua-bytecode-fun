use bytecode::{lua51::{Constants, Proto, instruction::{Opcode, Instr, Instruction}, serialize_bytecode, Header}};

use crate::control_flow::{self, Block};

pub type InstructionPointer = usize;
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Op {
	A,
	B,
	C,
	Bx,
	sBx
}

pub struct Context {
	pub header: Header,
	pub chunk: Proto,

	// state
	constant_refs: Vec<(u32, Op, InstructionPointer)>, // all instructions that reference a certain constant;
}

impl Context {
	pub fn new(header: Header, chunk: Proto) -> Self {
		let mut chunk = chunk;

		// not gonna bother fixing these when mutation of the IR occurs 
		chunk.source_lines = None;
		chunk.locals = None;
		chunk.upvals = None;

		Self {
			header,
			chunk: chunk,
			constant_refs: vec![]
		 }
	}

	// ** instruction & constant mapping and adding **

	// apply constant refs to instruction operands
	// shift instructions in constant refs when adding new instruction

	pub fn get_constant(&self, kst: u32) -> Option<&Constants> {
		self.chunk.constants.get(kst as usize)
	}
	pub fn add_constant_ref(&mut self, ip: InstructionPointer, kst: u32, op: Op) {
		// println!("kst: {:?} reference to ip: {}", kst, ip);
		self.constant_refs.push((kst, op, ip));
	}
	pub fn apply_constant_ref(&mut self) { 
		for (kst, op, ip) in &self.constant_refs {
			let instruction = self.chunk.instructions.get_mut(*ip);
			if let Some(instr) = instruction {
				// place new constants into instruction
				match &mut instr.1 {
					Instr::LoadK(_r, k)
					| Instr::GetGlobal(_r, k)
					| Instr::SetGlobal(_r, k) => {
						k.0 = *kst;
					},
					Instr::BinOp(_r, mut a_rk, _op, mut b_rk) => {
						match op {
							Op::B => a_rk.set(*kst),
							Op::C => b_rk.set(*kst),
							_ => unreachable!()
						}
					},
					Instr::BinCondOp(_r, mut a_rk, _op, mut b_rk) => {
						match op {
							Op::B => a_rk.set(*kst),
							Op::C => b_rk.set(*kst),
							_ => unreachable!()
						}
					},
					_ => {}
				}

				// update the cached opmode
				let new_mode = instr.1.get_opmode();
				instr.2 = new_mode;
			} else {
				panic!("loss in instruction position");
			}
		}
	}
	pub fn add_constant(&mut self, idx: usize, kst: Constants) {
		for v in self.constant_refs.iter_mut() {
			if (v.0 as usize) >= idx {
				println!("shifting instr #{}'s kst right 1", v.2);
				v.0 += 1;
			}
		}
		self.apply_constant_ref();
		self.chunk.constants.insert(idx, kst);
	}
	pub fn get_or_add_constant(&mut self, kst: Constants) -> u32 {
		let found_idx = self.chunk.constants.iter().position(|k| k == &kst);
		if let Some(idx) = found_idx {
			idx as u32
		} else {
			let len = self.chunk.constants.len();
			self.add_constant(len, kst);
			len as u32
		}
	}
	// maps all constants to self.constant_refs
	pub fn map_instr(&self, ip: usize, instruction: &Instruction) -> Vec<(usize, u32, Op)> {
		let mut refs = vec![];

		let instr = &instruction.1;
		match instr {
			Instr::LoadK(_r, k)
			| Instr::GetGlobal(_r, k)
			| Instr::SetGlobal(_r, k) => {
				if self.get_constant(k.0).is_some() {
					refs.push((ip, k.0, Op::Bx))
				}
			},
			Instr::BinOp(_r, a_rk, _op, b_rk) => {
				let ark = a_rk.get();
				let a_kst = self.get_constant(ark);
				if a_kst.is_some() {
					refs.push((ip, ark, Op::B));
				}

				let brk = b_rk.get();
				let b_kst = self.get_constant(brk);
				if b_kst.is_some() {
					refs.push((ip, brk, Op::C));
				}
			},
			Instr::BinCondOp(_r, a_rk, _op, b_rk) => {
				let ark = a_rk.get();
				let a_kst = self.get_constant(ark);
				if a_kst.is_some() {
					refs.push((ip, ark, Op::B));
				}

				let brk = b_rk.get();
				let b_kst = self.get_constant(brk);
				if b_kst.is_some() {
					refs.push((ip, brk, Op::C));
				}
			},
			_ => {}
		}
	
		refs
	}
	pub fn map_constants(&mut self) {
		let chunk = &self.chunk;
		let mut refs = vec![];

		chunk.instructions.iter().enumerate().for_each(|(ip, instr)| 
			refs.append(&mut self.map_instr(ip, instr))
		);

		for (ip, kst, op) in refs {
			self.add_constant_ref(ip, kst, op);
		}

	}
	pub fn add_instruction(&mut self, idx: InstructionPointer, instr: Instruction) -> usize {
		// hacky af
		let clone = instr.clone();
		let id = clone.3;

		// add instruction
		self.chunk.instructions.insert(idx, instr);

		// update references that references the old IP
		// println!("added instr, remapping constant refs");
		for ref_ in self.constant_refs.iter_mut() {
			if ref_.2 >= idx {
				ref_.2 += 1;
			}
		}

		// add instruction constant references, must go AFTER the updating references
		let r = self.map_instr(idx, &clone);
		for (ip, kst, op) in r {
			self.add_constant_ref(ip, kst, op);
		}

		// apply it to the instructions and not just references
		self.apply_constant_ref();

		// return unique id
		id
	}
	pub fn find_instruction_pt(&self, idx: usize) -> Option<InstructionPointer> {
		self.chunk.instructions.iter().position(|instr| instr.3 == idx)
	}
	
	//
	pub fn view(&self) {
		println!("INSTRUCTIONS : {:?}", &self.chunk.instructions.iter().map(|instr| &instr.1).collect::<Vec<&Instr>>());
		//println!("instr: {:#?}", self.chunk.instructions);
		println!("CONSTANTS : {:?}", &self.chunk.constants);
	}
	pub fn get_max_ip(&self) -> InstructionPointer {
		let len = self.chunk.instructions.len();
		if len > 0 {
			self.chunk.instructions.len()
		} else {
			0
		}
	}

	// ** control flow mapping **

	pub fn map_control_flow(&self) -> Vec<Block> {
		let blocks = control_flow::Mapper::new().map(&self.chunk.instructions);
		blocks
		// control_flow::ControlFlow::new(blocks, &self.chunk)
	}

	pub fn map(&mut self) {
		self.map_constants(); 

		// let hello = self.get_or_add_constant(Constants::String("SPIKE GAY".to_string()));
		// let print = self.get_or_add_constant(Constants::String("print".to_string()));
		// self.add_instruction(0, Instruction::new(Opcode::GetGlobal, Instr::GetGlobal(Reg(0), Kst(print))));
		// self.add_instruction(1, Instruction::new(Opcode::LoadK, Instr::LoadK(Reg(1), Kst(hello))));
		// self.add_instruction(2, Instruction::new(Opcode::Call, Instr::Call(Reg(0), 2, 1)));

		// self.view();
	}


	pub fn assemble(&self) -> Vec<u8> {
		serialize_bytecode(&self.header, &self.chunk)
	}
}