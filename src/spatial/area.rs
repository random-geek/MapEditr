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

	pub fn iterate(&self) -> AreaIterator {
		AreaIterator::new(self.min, self.max)
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

	pub fn contains_block(&self, block_pos: Vec3) -> bool {
		let corner = block_pos * 16;
		self.min.x <= corner.x && corner.x + 15 <= self.max.x
			&& self.min.y <= corner.y && corner.y + 15 <= self.max.y
			&& self.min.z <= corner.z && corner.z + 15 <= self.max.z
	}

	pub fn touches_block(&self, block_pos: Vec3) -> bool {
		let corner = block_pos * 16;
		self.min.x <= corner.x + 15 && corner.x <= self.max.x
			&& self.min.y <= corner.y + 15 && corner.y <= self.max.y
			&& self.min.z <= corner.z + 15 && corner.z <= self.max.z
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

	pub fn abs_block_overlap(&self, block_pos: Vec3) -> Option<Area> {
		let block_min = block_pos * 16;
		let block_max = block_min + 15;
		let node_min = Vec3 {
			x: max(self.min.x, block_min.x),
			y: max(self.min.y, block_min.y),
			z: max(self.min.z, block_min.z)
		};
		let node_max = Vec3 {
			x: min(self.max.x, block_max.x),
			y: min(self.max.y, block_max.y),
			z: min(self.max.z, block_max.z)
		};

		if node_min.x <= node_max.x
			&& node_min.y <= node_max.y
			&& node_min.z <= node_max.z
		{
			Some(Self {min: node_min, max: node_max})
		} else {
			None
		}
	}

	pub fn rel_block_overlap(&self, block_pos: Vec3) -> Option<Area> {
		let corner = block_pos * 16;
		let rel_min = self.min - corner;
		let rel_max = self.max - corner;
		let node_min = Vec3 {
			x: max(rel_min.x, 0),
			y: max(rel_min.y, 0),
			z: max(rel_min.z, 0)
		};
		let node_max = Vec3 {
			x: min(rel_max.x, 15),
			y: min(rel_max.y, 15),
			z: min(rel_max.z, 15)
		};

		if node_min.x <= node_max.x
			&& node_min.y <= node_max.y
			&& node_min.z <= node_max.z
		{
			Some(Self {min: node_min, max: node_max})
		} else {
			None
		}
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

		assert_eq!(
			Area::new(Vec3::new(0, 0, 0), Vec3::new(0, 0, 0)).volume(), 1);
		assert_eq!(
			Area::new(Vec3::new(0, -9, 14), Vec3::new(19, 0, 17)).volume(),
			800);
	}

	#[test]
	fn test_area_iteration() {
		fn iter_area(a: Area) {
			let mut iter = a.iterate();
			for z in a.min.z..=a.max.z {
				for y in a.min.y..=a.max.y {
					for x in a.min.x..=a.max.x {
						assert_eq!(iter.next(), Some(Vec3::new(x, y, z)))
					}
				}
			}
			assert_eq!(iter.next(), None);
		}

		iter_area(Area::new(Vec3::new(-1, -1, -1), Vec3::new(-1, -1, -1)));
		iter_area(Area::new(Vec3::new(10, -99, 11), Vec3::new(10, -99, 12)));
		iter_area(Area::new(Vec3::new(0, -1, -2), Vec3::new(5, 7, 11)));
	}

	#[test]
	fn test_area_containment() {
		let area = Area::new(Vec3::new(-1, -32, 16), Vec3::new(30, -17, 54));
		let contained = Area::new(Vec3::new(0, -2, 1), Vec3::new(0, -2, 2));
		let touching = Area::new(Vec3::new(-1, -2, 1), Vec3::new(1, -2, 3));

		for pos in Area::new(touching.min - 2, touching.max + 2).iterate() {
			assert_eq!(area.touches_block(pos), touching.contains(pos));
			assert_eq!(area.contains_block(pos), contained.contains(pos));
		}
	}

	#[test]
	fn test_area_block_overlap() {
		let area = Area::new(Vec3::new(-3, -3, -3), Vec3::new(15, 15, 15));
		let pairs = vec![
			(Vec3::new(-1, -1, -1),
				Some(Area::new(Vec3::new(13, 13, 13), Vec3::new(15, 15, 15)))),
			(Vec3::new(0, 0, 0),
				Some(Area::new(Vec3::new(0, 0, 0), Vec3::new(15, 15, 15)))),
			(Vec3::new(1, 1, 1), None),
			(Vec3::new(-1, 0, 0),
				Some(Area::new(Vec3::new(13, 0, 0), Vec3::new(15, 15, 15)))),
		];
		for pair in pairs {
			assert_eq!(area.rel_block_overlap(pair.0), pair.1);
		}
	}
}
