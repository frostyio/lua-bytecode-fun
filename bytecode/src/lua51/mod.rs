mod reader;
mod deserialize;
pub mod instruction;
mod writer;
mod serialize;
pub mod luac;

pub use reader::Reader;
pub use writer::Writer;
pub use deserialize::deserialize_bytecode;
pub use serialize::serialize_bytecode;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Constants {
	Nil,
	Boolean(bool),
	Number(f64),
	String(String)
}

#[derive(Debug, Clone)]
pub struct Local(String, u32, u32);

#[derive(Debug, Clone)]
pub struct Proto {
	pub source: String,
	pub line_defined: u32,
	pub last_line_defined: u32,
	pub nupvals: u8,
	pub nparams: u8,
	pub is_vararg_flag: u8,
	pub max_stack_size: u8,
	pub instructions: Vec<instruction::Instruction>,
	pub constants: Vec<Constants>,
	pub prototypes: Vec<Self>,
	pub source_lines: Option<Vec<u32>>,
	pub locals: Option<Vec<Local>>,
	pub upvals: Option<Vec<String>>
}

impl  Proto {
	pub fn default() -> Self {
		Self {
			source: "@default.lua".to_string(),
			line_defined: 0,
			last_line_defined: 0,
			nupvals: 0,
			nparams: 0,
			is_vararg_flag: 2,
			max_stack_size: 2,
			instructions: vec![], // return should be here, but is it required?
			constants: vec![],
			prototypes: vec![],
			source_lines: Some(vec![]),
			locals: Some(vec![]),
			upvals: Some(vec![])
		}
	}
}

pub type Header = (u8, u8, u8, u8); // int, size_t, instr, lua_number