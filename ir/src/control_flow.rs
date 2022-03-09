use std::{collections::{HashMap, BTreeSet}};

use bytecode::lua51::{instruction::{Instr, Instruction, Opcode}, Proto};

use crate::context::InstructionPointer;

#[derive(Debug)]
pub enum Target {
	Undefined(i32),
	Label(u32)
}

pub struct Mapper {
	possible_edges: BTreeSet<usize>,
	edges: HashMap<usize, i32>
}

#[derive(Debug)]
pub struct Block {
	pub code: Vec<InstructionPointer>,
	pub target: Target
}

// https://github.com/Rerumu/lau/blob/master/src/lua54/disassembler/splitter.rs
impl Mapper {
	pub fn new() -> Self {
		Self {
			possible_edges: BTreeSet::from([0]), // initial ip 0
			edges: HashMap::new()
		}
	}

	pub fn map(&mut self, code: &Vec<Instruction>) -> Vec<Block> {
		self.map_edges(&code);
		self.split_at_edges()
	}

	pub fn add_possible(&mut self, pc: usize, offset: i32) {
		self.possible_edges.insert((pc as i32 + offset + 1) as usize);
		self.edges.insert(pc, offset);
	}

	pub fn map_edges(&mut self, code: &Vec<Instruction>) {
		for (pc, instruction) in code.iter().enumerate() {
			match instruction.0 {
				Opcode::Test
				| Opcode::TestSet
				| Opcode::Eq
				| Opcode::Lt
				| Opcode::Le => {
					self.add_possible(pc, 1);
				}
				Opcode::Jump => {
					if let Instr::Jump(_, _b) = instruction.1 {
						//self.add_possible(pc - 1, b + 1)
						self.possible_edges.insert(pc);
						continue;
					}
				}
				// Opcode::Call => {
				// 	self.add_possible(pc, 1);
				// }
				Opcode::ForPrep => {
					if let Instr::ForPrep(_, _bx) = instruction.1 {
						// self.add_possible(pc, bx + 1);
						self.possible_edges.insert(pc);
						self.possible_edges.insert(pc + 1);
					}
				}
				Opcode::ForLoop => {
					if let Instr::ForLoop(_, _sbx) = instruction.1 {
						// self.add_possible(pc, sbx);
						self.possible_edges.insert(pc);
						self.possible_edges.insert(pc + 1);
					}
				}
				Opcode::NOP => {
					// self.add_possible(pc, 1);
				}
				Opcode::Call => {
					
				}
				// Opcode::Closure => {}
				Opcode::Return => {}
				_ => { continue }
			}
			self.possible_edges.insert(pc + 1);
		}
		self.possible_edges = std::mem::take(&mut self.possible_edges)
			.into_iter()
			.filter(|&v| v <= code.len())
			.collect();

		//println!("{:?}", self.possible_edges);
	}

	fn get_offsettarget(&self, post: usize, offset: i32) -> Target {
		let dest = post as i32 + offset;
		let index = self.possible_edges.range(0..=dest as usize).count();

		if index > self.possible_edges.len() {
			Target::Undefined(offset)
		} else {
			Target::Label(index as u32 - 1)
		}
	}

	fn split_at_edges(&self) -> Vec<Block> {
		let mut list: Vec<Block> = Vec::new();
		let mut prev = 0;

		for pc in self.possible_edges.iter().skip(1).map(|v| v - 1) {
			let offset = self.edges.get(&pc).copied().unwrap_or_default();
			let post = pc + 1;
			let code = (prev..post).collect::<Vec<usize>>();
			let target = self.get_offsettarget(post, offset);
			prev = post;
			list.push(Block { code, target });
		}

		list
	}
}

pub struct ControlFlow<'a>  {
	code: &'a Proto
}

impl<'a> ControlFlow<'a> {
	pub fn new(blocks: Vec<Block>, chunk: &'a Proto) -> Self {
		let s = Self {
			code: chunk,
		};

		blocks.into_iter().enumerate().for_each(|(t, b)| s.map_control(t as u32, b));

		s
	}

	pub fn map_control(&self, _target: u32, block: Block) {
		let last_pc = block.code.last().unwrap();
		let instr = &self.code.instructions[*last_pc];

		match instr.0 {
			Opcode::ForPrep => {

			},
			_ => {}
		}
	}
}