// handles creating & using registers

use rand::Rng;

pub type RegisterScalar = usize;

pub struct Registers {
	registers: Vec<RegisterScalar>, // index = a register, value = register position
	unused: Vec<usize>, // all unused registers
	possible: Vec<usize>,
}

impl Registers {
	pub fn from_size(capacity: RegisterScalar) -> Self {
		Self {
			registers: (0..capacity).collect(),
			unused: (0..capacity).collect(),
			possible: (0..capacity).collect()
		}
	}

	pub fn from_size_randomized(capacity: RegisterScalar) -> Self {
		let mut rng = rand::thread_rng();

		Self {
			registers: (0..capacity).map(|_| rng.gen_range(0..capacity)).collect(),
			unused: (0..capacity).collect(),
			possible: (0..capacity).collect()
		}
	}

	// gets a new registers
	pub fn new(&mut self) -> RegisterScalar {
		let reg_idx = self.unused.remove(0);
		self.possible.remove(0);
		let reg = self.registers[reg_idx];
		reg
	}


	pub fn get(&mut self, reg: RegisterScalar) -> RegisterScalar {
		let reg = self.registers[self.possible[reg]];
		reg
	}

	pub fn set(&mut self, idx: usize, reg: RegisterScalar) {
		self.registers[idx] = reg
	}

	pub fn offset(&mut self, offset: usize) {
		(0..offset).for_each(|_| { self.unused.remove(0); } );
		(0..offset).for_each(|_| { self.possible.remove(0); } );
	}
}