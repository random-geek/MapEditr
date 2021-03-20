mod vec3;
mod area;

pub use vec3::{MAP_LIMIT, Vec3};
pub use area::Area;


/// Iterates over all the block indices that are *not* contained within an
/// area, in order.
pub struct InverseBlockIterator {
	area: Area,
	idx: usize,
	can_skip: bool,
	skip_pos: Vec3,
	skip_idx: usize,
	skip_len: usize,
}

impl InverseBlockIterator {
	pub fn new(area: Area) -> Self {
		assert!(area.min.x >= 0 && area.max.x < 16
			&& area.min.y >= 0 && area.max.y < 16
			&& area.min.z >= 0 && area.max.z < 16);

		Self {
			area,
			idx: 0,
			can_skip: true,
			skip_pos: area.min,
			skip_idx:
				(area.min.x + area.min.y * 16 + area.min.z * 256) as usize,
			skip_len: (area.max.x - area.min.x + 1) as usize
		}
	}
}

impl Iterator for InverseBlockIterator {
	type Item = usize;

	fn next(&mut self) -> Option<Self::Item> {
		while self.can_skip && self.idx >= self.skip_idx {
			self.idx += self.skip_len;
			// Increment self.skip_pos, self.skip_idx.
			let mut sp = self.skip_pos;
			sp.y += 1;
			if sp.y > self.area.max.y {
				sp.y = self.area.min.y;
				sp.z += 1;
				if sp.z > self.area.max.z {
					// No more skips
					self.can_skip = false;
					break;
				}
			}
			self.skip_pos = sp;
			self.skip_idx = (sp.x + sp.y * 16 + sp.z * 256) as usize;
		}

		if self.idx < 4096 {
			let idx = self.idx;
			self.idx += 1;
			Some(idx)
		} else {
			None
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_inverse_block_iterator() {
		let dim_pairs = [
			(1, 14), // Touching neither end
			(1, 2),
			(9, 9),
			(1, 15), // Touching max end
			(11, 15),
			(15, 15),
			(0, 0), // Touching min end
			(0, 1),
			(0, 14),
			(0, 15), // End-to-end
		];

		fn test_area(area: Area) {
			let mut iter = InverseBlockIterator::new(area);
			for pos in &Area::new(Vec3::new(0, 0, 0), Vec3::new(15, 15, 15)) {
				if !area.contains(pos) {
					let idx = (pos.x + pos.y * 16 + pos.z * 256) as usize;
					assert_eq!(iter.next(), Some(idx));
				}
			}
			assert_eq!(iter.next(), None)
		}

		for z_dims in &dim_pairs {
			for y_dims in &dim_pairs {
				for x_dims in &dim_pairs {
					let area = Area::new(
						Vec3::new(x_dims.0, y_dims.0, z_dims.0),
						Vec3::new(x_dims.1, y_dims.1, z_dims.1)
					);
					test_area(area);
				}
			}
		}
	}
}
