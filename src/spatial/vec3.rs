#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
	pub x: i32,
	pub y: i32,
	pub z: i32
}

impl Vec3 {
	#[inline]
	pub fn new(x: i32, y: i32, z: i32) -> Self {
		Self {x, y, z}
	}

	pub fn from_block_key(key: i64) -> Self {
		let x = (key + 2048).rem_euclid(4096) - 2048;
		let rem = (key - x) / 4096;
		let y = (rem + 2048).rem_euclid(4096) - 2048;
		let z = (rem - y) / 4096;
		Self {x: x as i32, y: y as i32, z: z as i32}
	}

	pub fn to_block_key(&self) -> i64 {
		// Make sure values are within range.
		assert!(-2048 <= self.x && self.x < 2048
			&& -2048 <= self.y && self.y < 2048
			&& -2048 <= self.z && self.z < 2048);

		self.x as i64
			+ self.y as i64 * 4096
			+ self.z as i64 * 4096 * 4096
	}

	pub fn from_u16_key(key: u16) -> Self {
		Self {
			x: (key & 0xF) as i32,
			y: ((key >> 4) & 0xF) as i32,
			z: ((key >> 8) & 0xF) as i32
		}
	}

	pub fn is_valid_block_pos(&self) -> bool {
		const LIMIT: i32 = 31000 / 16;

		-LIMIT <= self.x && self.x <= LIMIT
			&& -LIMIT <= self.y && self.y <= LIMIT
			&& -LIMIT <= self.z && self.z <= LIMIT
	}

	pub fn is_valid_node_pos(&self) -> bool {
		const LIMIT: i32 = 31000;

		-LIMIT <= self.x && self.x <= LIMIT
			&& -LIMIT <= self.y && self.y <= LIMIT
			&& -LIMIT <= self.z && self.z <= LIMIT
	}

	pub fn map<F>(&self, func: F) -> Self
		where F: Fn(i32) -> i32
	{
		Self {
			x: func(self.x),
			y: func(self.y),
			z: func(self.z)
		}
	}
}

impl std::ops::Add<Self> for Vec3 {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
			z: self.z + rhs.z
		}
	}
}

impl std::ops::Add<i32> for Vec3 {
	type Output = Self;

	fn add(self, rhs: i32) -> Self {
		Self {
			x: self.x + rhs,
			y: self.y + rhs,
			z: self.z + rhs
		}
	}
}

impl std::ops::Sub<Self> for Vec3 {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
			z: self.z - rhs.z
		}
	}
}

impl std::ops::Sub<i32> for Vec3 {
	type Output = Self;

	fn sub(self, rhs: i32) -> Self {
		Self {
			x: self.x - rhs,
			y: self.y - rhs,
			z: self.z - rhs
		}
	}
}

impl std::ops::Mul<Self> for Vec3 {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self {
		Self {
			x: self.x * rhs.x,
			y: self.y * rhs.y,
			z: self.z * rhs.z
		}
	}
}

impl std::ops::Mul<i32> for Vec3 {
	type Output = Self;

	fn mul(self, rhs: i32) -> Self {
		Self {
			x: self.x * rhs,
			y: self.y * rhs,
			z: self.z * rhs
		}
	}
}

impl std::fmt::Display for Vec3 {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "({}, {}, {})", self.x, self.y, self.z)
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_vec3() {
		assert_eq!(Vec3::new(42, 0, -6000), Vec3 {x: 42, y: 0, z: -6000});

		assert_eq!(Vec3::new(-31000, 50, 31000).is_valid_node_pos(), true);
		assert_eq!(Vec3::new(-31000, -11, 31001).is_valid_node_pos(), false);

		assert_eq!(Vec3::new(-1937, -5, 1101).is_valid_block_pos(), true);
		assert_eq!(Vec3::new(-1937, 1938, -10).is_valid_block_pos(), false);
		assert_eq!(Vec3::new(-1938, 4, 1900).is_valid_block_pos(), false);

		let exp = 3;
		assert_eq!(Vec3::new(-3, 4, 10).map(|n| n.pow(exp)),
			Vec3::new(-27, 64, 1000));

		assert_eq!(format!("{}", Vec3::new(-1000, 0, 70)), "(-1000, 0, 70)");
	}

	#[test]
	fn test_vec3_conversions() {
		/* Test block key/vector conversions */
		const Y_FAC: i64 = 0x1_000;
		const Z_FAC: i64 = 0x1_000_000;
		let bk_pairs = [
			// Basics
			(Vec3::new(0, 0, 0), 0),
			(Vec3::new(1, 0, 0), 1),
			(Vec3::new(0, 1, 0), 1 * Y_FAC),
			(Vec3::new(0, 0, 1), 1 * Z_FAC),
			// X/Y/Z Boundaries
			(Vec3::new(-2048, 0, 0), -2048),
			(Vec3::new(2047, 0, 0), 2047),
			(Vec3::new(0, -2048, 0), -2048 * Y_FAC),
			(Vec3::new(0, 2047, 0), 2047 * Y_FAC),
			(Vec3::new(0, 0, -2048), -2048 * Z_FAC),
			(Vec3::new(0, 0, 2047), 2047 * Z_FAC),
			// Extra spicy boundaries
			(Vec3::new(-42, 2047, -99), -42 + 2047 * Y_FAC + -99 * Z_FAC),
			(Vec3::new(64, -2048, 22), 64 + -2048 * Y_FAC + 22 * Z_FAC),
			(Vec3::new(2047, 555, 35), 2047 + 555 * Y_FAC + 35 * Z_FAC),
			(Vec3::new(-2048, 600, -70), -2048 + 600 * Y_FAC + -70 * Z_FAC),
			// Multiple boundaries
			(Vec3::new(2047, -2048, 16), 2047 + -2048 * Y_FAC + 16 * Z_FAC),
			(Vec3::new(-2048, 2047, 50), -2048 + 2047 * Y_FAC + 50 * Z_FAC),
		];

		for pair in &bk_pairs {
			assert_eq!(pair.0.to_block_key(), pair.1);
			assert_eq!(pair.0, Vec3::from_block_key(pair.1));
		}

		/* Test u16/vector conversions */
		let u16_pairs = [
			(Vec3::new(0, 0, 0), 0x000),
			(Vec3::new(1, 0, 0), 0x001),
			(Vec3::new(0, 1, 0), 0x010),
			(Vec3::new(0, 0, 1), 0x100),
			(Vec3::new(15, 15, 15), 0xFFF),
			(Vec3::new(5, 15, 9), 0x9F5)
		];

		for pair in &u16_pairs {
			assert_eq!(pair.0, Vec3::from_u16_key(pair.1));
		}
	}
}
