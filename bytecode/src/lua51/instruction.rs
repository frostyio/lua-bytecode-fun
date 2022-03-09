use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use std::sync::atomic::{AtomicUsize, Ordering};
static COUNTER: AtomicUsize = AtomicUsize::new(1);

#[derive(FromPrimitive, Debug, Clone, Copy)]
pub enum Opcode {
	Move = 0,
	LoadK,
	LoadBool,
	LoadNil,
	GetUpval,
	GetGlobal,
	GetTable,
	SetGlobal,
	SetUpval,
	SetTable,
	NewTable,
	Self_,
	Add,
	Sub,
	Mul,
	Div,
	Mod,
	Pow,
	Unm,
	Not,
	Len,
	Concat,
	Jump,
	Eq,
	Lt,
	Le,
	Test,
	TestSet,
	Call,
	TailCall,
	Return,
	ForLoop,
	ForPrep,
	TForLoop,
	SetList,
	Close,
	Closure,
	VarArg,
	NOP
}

impl Opcode {
	fn from_instr(instr: u32) -> Self {
		Self::from_u8((instr & 0x3f) as u8).unwrap()
	}

	fn to_instr(&self) -> u32 {
		*self as u32
	}
}

struct Abc(u8, u16, u16);
impl Abc {
	fn bool_a(&self) -> bool { self.0 != 0 }
	fn bool_b(&self) -> bool { self.1 != 0 }
	fn bool_c(&self) -> bool { self.2 != 0 }

	fn reg_a(&self) -> Reg { Reg(self.0) }
	fn reg_b(&self) -> Reg {
		if self.1 <= 0xff {
			return Reg(self.1 as u8)
		}
		panic!()
	}
	fn reg_c(&self) -> Reg {
		if self.2 <= 0xff {
			return Reg(self.2 as u8)
		}
		panic!()
	}

	fn rk_b(&self) -> RegKst {
		if self.1 > 0xff {
			//println!("possible subtraction needed #1");
			RegKst::K(Kst((self.1) as u32)) // constants should be subtracted 0x100 for value 
		} else {
			RegKst::R(Reg(self.1 as u8))
		}
	}
	fn rk_c(&self) -> RegKst {
		if self.2 > 0xff {
			//println!("possible subtraction needed #1");
			RegKst::K(Kst((self.2) as u32)) // constants should be subtracted 0x100 for value 
		} else {
			RegKst::R(Reg(self.2 as u8))
		}
	}
}
#[allow(non_snake_case)]
fn ABC(instr: u32) -> Abc {
	Abc (
		((instr >> (6)) & 0xff) as u8,
		((instr >> (6 + 8 + 9)) & 0x1ff) as u16,
		((instr >> (6 + 8)) & 0x1ff) as u16,
	)
}

struct Abx(u8, u32);
impl Abx {
	fn kst(&self) -> Kst { Kst(self.1) }
}
#[allow(non_snake_case)]
fn ABx(instr: u32) -> Abx {
	Abx (
		((instr >> 6) & 0xff) as u8,
		((instr >> (6 + 8)) & 0x3ffff) as u32
	)
}

#[derive(Debug)]
struct Asbx(u8, i32);
#[allow(non_snake_case)]
fn AsBx(instr: u32) -> Asbx {
	Asbx (
		((instr >> 6) & 0xff) as u8,
		((instr >> (6 + 8)) & 0x3ffff) as i32 - 0x1ffff
	)
}

// fn fb2int(x: u16) -> u32 {
// 	let e = (x >> 3) & 31;
// 	if e == 0 {
// 		x.into()
// 	} else {
// 		(((x & 7) + 8) as u32) << (e - 1)
// 	}
// }

#[derive(Debug, Clone, Copy)]
pub struct Reg(pub u8);

#[derive(Debug, Clone, Copy)]
pub struct Kst(pub u32);

#[derive(Debug, Clone, Copy)]
pub enum RegKst {
	R(Reg), K(Kst)
}
impl RegKst {
	pub fn get(&self) -> u32 {
		match self {
			Self::R(r) => r.0 as u32,
			Self::K(kst) => {
				// println!("possible subtraction needed #2");
				kst.0
			}
		}
	}
	pub fn set(&mut self, v: u32) {
		match self {
			Self::R(r) => r.0 = v as u8,
			Self::K(kst) => kst.0 = v
		}
	}
}

pub type Upvalue = u16;

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
	Add, Sub, Mul, Div, Mod, Pow
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
	Unm, Not, Len
}

#[derive(Debug, Clone, Copy)]
pub enum BinCondOp {
	Eq, Lt, Le
}

#[derive(Debug, Clone)]
pub enum Instr {
	Move(Reg, Reg),
	LoadK(Reg, Kst),
	LoadBool(Reg, bool, bool),
	LoadNil(Reg, Reg),
	GetUpval(Reg, Upvalue),
	GetGlobal(Reg, Kst),
	GetTable(Reg, Reg, RegKst),
	SetGlobal(Reg, Kst),
	SetUpval(Reg, Upvalue),
	SetTable(Reg, RegKst, RegKst),
	NewTable(Reg, Reg, Reg),
	Self_(Reg, Reg, RegKst),
	BinOp(Reg, RegKst, BinOp, RegKst),
	UnOp(Reg, UnOp, Reg),
	Concat(Reg, Reg, Reg),
	Jump(Reg, i32),
	BinCondOp(bool, RegKst, BinCondOp, RegKst),
	Test(Reg, bool),
	TestSet(Reg, Reg, bool),
	Call(Reg, u16, u16),
	TailCall(Reg, u16, u16),
	Return(Reg, u16),
	ForLoop(Reg, i32),
	ForPrep(Reg, i32),
	TForLoop(Reg, u16),
	SetList(Reg, u16, u32),
	Close(Reg),
	Closure(Reg, u32), // Vec<u32>), //??
	VarArg(Reg, i32),
	NOP
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Opmode {
	iABC(u32, u32, u32),
	iABx(u32, u32),
	iAsBx(u32, u32),
	NOP
}

impl Instr {
	pub fn from_opmode(op: Opcode, opmode: Opmode) -> Self {
		if let Opmode::NOP = opmode {
			return Self::NOP;
		}

		let serialized = Instruction(op, Instr::NOP, opmode, 0).serialize();
		Self::from_instr(serialized, None)
	}

	pub fn from_instr(instr: u32, next_instr: Option<&u32>) -> Self {
		let op = Opcode::from_instr(instr);
		let abc = ABC(instr);
		let abx = ABx(instr);
		let asbx = AsBx(instr);

		match op {
			Opcode::Move => Self::Move(abc.reg_a(), abc.reg_b()), // iABC
			Opcode::LoadK => Self::LoadK(abc.reg_a(), abx.kst()), // iABx
			Opcode::LoadBool => Self::LoadBool(abc.reg_a(), abc.bool_b(), abc.bool_c()), // iABC
			Opcode::LoadNil => Self::LoadNil(abc.reg_a(), abc.reg_b()), // iABC
			Opcode::GetUpval => Self::GetUpval(abc.reg_a(), abc.1), // iABC
			Opcode::GetGlobal => Self::GetGlobal(abc.reg_a(), abx.kst()), // iABx
			Opcode::GetTable => Self::GetTable(abc.reg_a(), abc.reg_b(), abc.rk_c()), // iABC
			Opcode::SetGlobal => Self::SetGlobal(abc.reg_a(), abx.kst()), // iABx
			Opcode::SetUpval => Self::SetUpval(abc.reg_a(), abc.1), // iABC
			Opcode::SetTable => Self::SetTable(abc.reg_a(), abc.rk_b(), abc.rk_c()), // iABC
			Opcode::NewTable => Self::NewTable(abc.reg_a(), abc.reg_b(), abc.reg_c()), // iABC
			Opcode::Self_ => Self::Self_(abc.reg_a(), abc.reg_b(), abc.rk_c()), // iABC
			Opcode::Add => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Add, abc.rk_c()), // iABC
			Opcode::Sub => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Sub, abc.rk_c()), // iABC
			Opcode::Mul => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Mul, abc.rk_c()), // iABC
			Opcode::Div => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Div, abc.rk_c()), // iABC
			Opcode::Mod => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Mod, abc.rk_c()), // iABC
			Opcode::Pow => Self::BinOp(abc.reg_a(), abc.rk_b(), BinOp::Pow, abc.rk_c()), // iABC
			Opcode::Unm => Self::UnOp(abc.reg_a(), UnOp::Unm, abc.reg_b()), // iABC
			Opcode::Not => Self::UnOp(abc.reg_a(), UnOp::Not, abc.reg_b()), // iABC
			Opcode::Len => Self::UnOp(abc.reg_a(), UnOp::Len, abc.reg_b()), // iABC
			Opcode::Concat => Self::Concat(abc.reg_a(), abc.reg_b(), abc.reg_c()), // iABC
			Opcode::Jump => Self::Jump(abc.reg_a(), asbx.1 as i32), // iAsBx
			Opcode::Eq => Self::BinCondOp(abc.bool_a(), abc.rk_b(), BinCondOp::Eq, abc.rk_c()), // iABC
			Opcode::Lt => Self::BinCondOp(abc.bool_a(), abc.rk_b(), BinCondOp::Lt, abc.rk_c()), // iABC
			Opcode::Le => Self::BinCondOp(abc.bool_a(), abc.rk_b(), BinCondOp::Le, abc.rk_c()), // iABC
			Opcode::Test => Self::Test(abc.reg_a(), abc.bool_c()), // iABC
			Opcode::TestSet => Self::TestSet(abc.reg_a(), abc.reg_b(), abc.bool_c()), // iABC
			Opcode::Call => Self::Call(abc.reg_a(), abc.1, abc.2), // iABC
			Opcode::TailCall => Self::TailCall(abc.reg_a(), abc.1, abc.2), // iABC
			Opcode::Return => Self::Return(abc.reg_a(), abc.1), // iABC
			Opcode::ForLoop => Self::ForLoop(abc.reg_a(), asbx.1 as i32), // iAsBx
			Opcode::ForPrep => Self::ForPrep(abc.reg_a(), asbx.1 as i32), // iAsBx
			Opcode::TForLoop => Self::TForLoop(abc.reg_a(), abc.2), // iABC
			Opcode::SetList => { // iABC
				let set = if abc.2 == 0 {
					// count += 1;
					*next_instr.unwrap()
				} else {
					abc.2.into()
				};
				Self::SetList(abc.reg_a(), abc.1, set)
			},
			Opcode::Close => Self::Close(abc.reg_a()), // iABC
			Opcode::Closure => Self::Closure(abc.reg_a(), abx.1),  // iABx
			Opcode::VarArg => Self::VarArg(abc.reg_a(), abc.1.into()), // iABC
			Opcode::NOP => Self::NOP
		}
	}
	pub fn get_opmode(&self) -> Opmode {
		match self {
			Self::Move(a, b) 
			| Self::LoadNil(a, b) => Opmode::iABC(a.0 as u32, b.0 as u32, 0),
			Self::LoadK(a, kst)
		 	| Self::GetGlobal(a, kst)
			| Self::SetGlobal(a, kst) => Opmode::iABx(a.0 as u32, kst.0),
			Self::LoadBool(a, b, c) => Opmode::iABC(a.0 as u32, *b as u32, *c as u32),
			Self::GetUpval(a, b) => Opmode::iABC(a.0 as u32, *b as u32, 0),
			Self::GetTable(a, b, c)
			| Self::Self_(a, b, c) => Opmode::iABC(a.0 as u32, b.0 as u32, c.get()),
			Self::SetUpval(a, b) => Opmode::iABC(a.0 as u32, *b as u32, 0),
			Self::SetTable(a, b, c) => Opmode::iABC(a.0 as u32, b.get(), c.get()),
			Self::NewTable(a, b, c) => Opmode::iABC(a.0 as u32, b.0 as u32, c.0 as u32),
			Self::BinOp(a, b, _, c) => Opmode::iABC(a.0 as u32, b.get(), c.get()),
			Self::UnOp(a, _, b) => Opmode::iABC(a.0 as u32, b.0 as u32, 0),
			Self::Concat(a, b, c) => Opmode::iABC(a.0 as u32, b.0 as u32, c.0 as u32),
			Self::Jump(a, b) => Opmode::iAsBx(a.0 as u32, *b as u32),
			Self::BinCondOp(a, b, _, c) => Opmode::iABC(*a as u32, b.get(), c.get()),
			Self::Test(a, c) => Opmode::iABC(a.0 as u32, 0, *c as u32),
			Self::TestSet(a, b, c) => Opmode::iABC(a.0 as u32, b.0 as u32, *c as u32),
			Self::Call(a, b, c)
			| Self::TailCall(a, b, c) => Opmode::iABC(a.0 as u32, *b as u32, *c as u32),
			Self::Return(a, b) =>  Opmode::iABC(a.0 as u32, *b as u32, 0),
			Self::TForLoop(a, b) => Opmode::iABC(a.0 as u32, 0, *b as u32),
			Self::ForLoop(a, b)
			| Self::ForPrep(a, b) => Opmode::iAsBx(a.0 as u32, *b as u32),
			Self::SetList(a, b, c) => Opmode::iABC(a.0 as u32, *b as u32, *c),
			Self::Close(a) => Opmode::iABC(a.0 as u32, 0, 0),
			Self::Closure(a, b) => Opmode::iABx(a.0 as u32, *b),
			Self::VarArg(a, b) => Opmode::iABC(a.0 as u32, *b as u32, 0),
			Self::NOP => Opmode::NOP
		}
	}
}

// unique id
fn get_id() -> usize { 
	COUNTER.fetch_add(1, Ordering::Relaxed) 
}

#[derive(Debug, Clone)]
pub struct Instruction(pub Opcode, pub Instr, pub Opmode, pub usize);
impl Instruction {
	pub fn new(op: Opcode, instr: Instr) -> Self {
		let mode = instr.get_opmode();
		Self(op, instr, mode, get_id())
	}

	pub fn from_instr(instr: u32, next_instr: Option<&u32>) -> Self {
		let op = Opcode::from_instr(instr);
		let inst = Instr::from_instr(instr, next_instr);
		let mode = inst.get_opmode();
		Self(op, inst, mode, get_id())
	}

	pub fn serialize(&self) -> u32 {
		let opmode = self.2;

		let mut serialized = self.0.to_instr();
		match opmode {
			Opmode::iABC(a, b, c) => serialized = serialized | ((a & 0xff) << 6) | ((b & 0x1ff) << (6 + 8 + 9)) | ((c & 0x1ff) << (6 + 8)),
			Opmode::iABx(a, bx) => serialized |= ((a & 0xff) << 6) | (bx << (6 + 8)),
			Opmode::iAsBx(a, sbx) => serialized |= ((a & 0xff) << 6) | ((((sbx as i32) + 0x1ffff) as u32) << 14),
			Opmode::NOP => panic!()
		}

		serialized
	}
}