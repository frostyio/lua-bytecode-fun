pub type Bytecode = [u8];

pub mod lua51;

pub trait Hex: std::fmt::Debug {
	fn hex(&self, pt: Option<usize>) -> String;
}

impl Hex for [u8] {
	fn hex(&self, pt: Option<usize>) -> String {
		let mut c: i8 = 0;
		let mut vec: Vec<String> = Vec::new();

		for (i, byte) in self.iter().enumerate() {
			c += 1;
			if c > 16 { 
				vec.push(String::from("\n")); 
				c = 1;
			}

			let mut hex = format!("{:02X?} ", byte);
			if pt.is_some() && i == pt.unwrap() {
				hex = format!("[ {}] ", hex);
			}
			vec.push(hex);
		}
		vec.push(String::from("\n"));

		format!("{}", vec.join(""))
	}
}