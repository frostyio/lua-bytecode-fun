use bytecode::lua51::{instruction::{BinCondOp, Instr, RegKst}, Constants};

enum DebugValue {
	Closure(usize),
	Block(usize),
	GotoBlock(usize, usize),
	IfStatement(RegKst, BinCondOp, RegKst),
	ForPrep(usize),
	ForLoop(usize)
}

fn get_v_from_rk<'a>(rk: &RegKst, ksts: &'a Vec<Constants>) -> String {
	match rk {
		RegKst::K(kst) => {
			let v = rk.get();
			if v > 0xff {
				format!("{:?}", &ksts[(v - 0x100) as usize])
			} else {
				format!("{:?}", &ksts[v as usize]) // should never happen
			}
		},
		RegKst::R(r) => {
			format!("{:?}", r)
		}
	}
}

pub struct Debug {
	last_block: usize,
	log: Vec<DebugValue>,
	instr_log: Vec<Vec<Instr>>,
	ip: usize
}

impl Debug {
	pub fn new() -> Self {
		Self {
			last_block: 0,
			log: vec![],
			instr_log: vec![],
			ip: 0
		}
	}

	fn add_log(&mut self, value: DebugValue) {
		self.log.push(value)
	}

	pub fn create_closure(&mut self, id: usize) {
		self.add_log(DebugValue::Closure(id))
	}

	pub fn create_block(&mut self, id: usize) {
		self.add_log(DebugValue::Block(id))
	}

	pub fn goto_block(&mut self, from_id: usize, target_id: usize) {
		self.add_log(DebugValue::GotoBlock(from_id, target_id))
	}

	pub fn for_prep(&mut self, target: usize) {
		self.add_log(DebugValue::ForPrep(target))
	}
	pub fn for_loop(&mut self, block: usize) {
		self.add_log(DebugValue::ForLoop(block))
	}

	pub fn if_statement(&mut self, kst1: RegKst, cond: BinCondOp, kst2: RegKst) {
		self.add_log(DebugValue::IfStatement(kst1, cond, kst2))
	}

	pub fn view(&mut self, constants: &Vec<Constants>) {
		let mut output: Vec<String> = vec![];

		for value in &self.log {
			match value {
				DebugValue::Closure(id) => output.push(format!("New closure at closure_id: {id}\n")),
				DebugValue::Block(id) => {
					output.push(format!("\tNew block at {id}\n"));
					// self.last_block = *id;
				},
				DebugValue::GotoBlock(from_id, target_id) => {
					output.push(format!("\t\tGoto block from {from_id} to {target_id}\n"))
				},
				DebugValue::IfStatement(kst1, cond, kst2) => {
					output.push(format!("\t\tIf {} {:?} {} then\n", get_v_from_rk(kst1, constants), cond, get_v_from_rk(kst2, constants)));
				},
				DebugValue::ForPrep(block) => {
					output.push(format!("\t\tForPrep to {}\n", block));
				}
				DebugValue::ForLoop(block) => {
					output.push(format!("\t\tForLoop to {}\n", block));
				}
				_ => {}
			}
		}

		println!("\n{}\n", output.concat());
	}
}