#[derive(Copy, Clone, Debug, PartialEq)]
pub struct V3f {
	pub x: f32,
	pub y: f32,
	pub z: f32
}

impl V3f {
	pub fn new(x: f32, y: f32, z: f32) -> Self {
		Self {x, y, z}
	}
}

impl std::ops::Add<Self> for V3f {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
			z: self.z + rhs.z
		}
	}
}

impl std::ops::Add<f32> for V3f {
	type Output = Self;

	fn add(self, rhs: f32) -> Self {
		Self {
			x: self.x + rhs,
			y: self.y + rhs,
			z: self.z + rhs
		}
	}
}

impl std::ops::Sub<Self> for V3f {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
			z: self.z - rhs.z
		}
	}
}

impl std::ops::Mul<Self> for V3f {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self {
		Self {
			x: self.x * rhs.x,
			y: self.y * rhs.y,
			z: self.z * rhs.z
		}
	}
}

impl std::ops::Mul<f32> for V3f {
	type Output = Self;

	fn mul(self, rhs: f32) -> Self {
		Self {
			x: self.x * rhs,
			y: self.y * rhs,
			z: self.z * rhs
		}
	}
}
