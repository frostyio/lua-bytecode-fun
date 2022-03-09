pub struct Reader<'a>(&'a [u8], usize);

impl<'a> Reader<'a> {
	pub fn new(buffer: &'a [u8]) -> Self {
		Self (buffer, 0)
	}

	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}

	#[inline]
	pub fn byte(&mut self) -> &u8 {
		let v = &self.0[self.1];
		self.1 += 1;
		v
	}

	#[inline]
	pub fn bytes(&mut self, n: usize) -> &[u8] {
		let v = &self.0[self.1..self.1 + n];
		self.1 += n;
		v
	}

	#[inline]
	pub fn int(&mut self, n: usize) -> u32 {
		let bytes = self.bytes(n);
		let mut sum: u32 = 0;
		for i in (0..n).rev() {
			if i == n {
				continue
			}

			sum = (sum << 8) + (bytes[i] as u32);
		}
		sum
	}

	#[inline]
	pub fn number(&mut self, int: usize) -> f64 {
		let a = self.int(int) as u64;
		let a2 = self.int(int) as u64;
		let b = (a2 << 32) | a;
		f64::from_bits(b)
	}

	#[inline]
	pub fn string(&mut self, size_t: u8) -> String {
		let str_size = self.int(size_t as usize);
		let mut str = self.bytes(str_size as usize).to_vec();
		str.pop(); // remove nul character
		String::from_utf8(str).expect("invalid string")
	}
}
