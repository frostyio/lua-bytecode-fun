pub struct Writer(Vec<u8>);
impl Writer {
	pub fn new() -> Self {
		Self(vec![])
	}

	pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
		&self.0.as_slice()
	}

	#[inline]
	pub fn byte(&mut self, b: u8) {
		self.0.push(b);
	}

	#[inline]
	pub fn bytes(&mut self, bs: Vec<u8>) {
		for b in bs {
			self.0.push(b)
		}
	}

	#[inline]
	pub fn int(&mut self, n: u32, s: u8) {
		let mut num = n;

		for _i in (0..s).rev() {
			self.byte((num & 0xff) as u8);
			num = num >> 8; 
		}
	}

	#[inline]
	pub fn string(&mut self, str: &str, s: u8) {
		self.int(str.len() as u32 + 1, s);
		self.bytes(str.as_bytes().to_vec());
		self.byte(0);
	}

	#[inline]
	pub fn number(&mut self, n: f64, s: u8) {
		let num = n.to_bits();
		let a0 = (num >> 32) as u32;
		let a1 = num as u32;
		self.int(a1, s);
		self.int(a0, s);
	}
}