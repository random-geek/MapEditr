use std::cmp::{min, max};

use super::Vec3;


pub struct AreaIterator {
	min: Vec3,
	max: Vec3,
	pos: Vec3
}

impl AreaIterator {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self {min, max, pos: min}
	}
}

impl Iterator for AreaIterator {
	type Item = Vec3;

	fn next(&mut self) -> Option<Self::Item> {
		if self.pos.z > self.max.z {
			None
		} else {
			let last_pos = self.pos;

			self.pos.x += 1;
			if self.pos.x > self.max.x {
				self.pos.x = self.min.x;
				self.pos.y += 1;
				if self.pos.y > self.max.y {
					self.pos.y = self.min.y;
					self.pos.z += 1;
				}
			}

			Some(last_pos)
		}
	}
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Area {
	pub min: Vec3,
	pub max: Vec3
}

impl Area {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		assert!(min.x <= max.x
			&& min.y <= max.y
			&& min.z <= max.z);
		Self {min, max}
	}

	pub fn from_unsorted(a: Vec3, b: Vec3) -> Self {
		Area {
			min: Vec3 {
				x: min(a.x, b.x),
				y: min(a.y, b.y),
				z: min(a.z, b.z)
			},
			max: Vec3 {
				x: max(a.x, b.x),
				y: max(a.y, b.y),
				z: max(a.z, b.z)
			}
		}
	}

	pub fn volume(&self) -> u64 {
		(self.max.x - self.min.x + 1) as u64 *
		(self.max.y - self.min.y + 1) as u64 *
		(self.max.z - self.min.z + 1) as u64
	}

	pub fn contains(&self, pos: Vec3) -> bool {
		self.min.x <= pos.x && pos.x <= self.max.x
			&& self.min.y <= pos.y && pos.y <= self.max.y
			&& self.min.z <= pos.z && pos.z <= self.max.z
	}

	pub fn iterate(&self) -> AreaIterator {
		AreaIterator::new(self.min, self.max)
	}

	pub fn to_contained_block_area(&self) -> Self {
		let min = Vec3::new(
			(self.min.x + 15).div_euclid(16),
			(self.min.y + 15).div_euclid(16),
			(self.min.z + 15).div_euclid(16)
		);
		let max = Vec3::new(
			(self.max.x - 15).div_euclid(16),
			(self.max.y - 15).div_euclid(16),
			(self.max.z - 15).div_euclid(16)
		);
		Self {min, max}
	}

	pub fn to_touching_block_area(&self) -> Self {
		let min = Vec3::new(
			self.min.x.div_euclid(16),
			self.min.y.div_euclid(16),
			self.min.z.div_euclid(16)
		);
		let max = Vec3::new(
			self.max.x.div_euclid(16),
			self.max.y.div_euclid(16),
			self.max.z.div_euclid(16)
		);
		Self {min, max}
	}
}

impl std::ops::Add<Vec3> for Area {
	type Output = Self;
	fn add(self, rhs: Vec3) -> Self {
		Area {
			min: self.min + rhs,
			max: self.max + rhs
		}
	}
}

impl std::ops::Sub<Vec3> for Area {
	type Output = Self;
	fn sub(self, rhs: Vec3) -> Self {
		Area {
			min: self.min - rhs,
			max: self.max - rhs
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_area() {
		assert_eq!(
			Area::from_unsorted(Vec3::new(8, 0, -10), Vec3::new(-8, 0, 10)),
			Area::new(Vec3::new(-8, 0, -10), Vec3::new(8, 0, 10))
		);
		assert_eq!(
			Area::from_unsorted(Vec3::new(10, 80, 42), Vec3::new(10, -50, 99)),
			Area::new(Vec3::new(10, -50, 42), Vec3::new(10, 80, 99))
		);
	}

	#[test]
	fn test_area_iteration() {
		let a = Area::new(Vec3::new(0, -1, -2), Vec3::new(5, 7, 11));
		let mut iter = a.iterate();

		for z in -2..=11 {
			for y in -1..=7 {
				for x in 0..=5 {
					assert_eq!(iter.next(), Some(Vec3::new(x, y, z)));
				}
			}
		}

		assert_eq!(iter.next(), None);
	}
}
