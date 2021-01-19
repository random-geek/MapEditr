use std::cmp::{min, max};

mod vec3;
// TODO
// mod v3f;
mod area;

pub use vec3::Vec3;
// pub use v3f::V3f;
pub use area::Area;


pub fn area_contains_block(area: &Area, block_pos: Vec3) -> bool {
	let corner = block_pos * 16;
	area.min.x <= corner.x && corner.x + 15 <= area.max.x
		&& area.min.y <= corner.y && corner.y + 15 <= area.max.y
		&& area.min.z <= corner.z && corner.z + 15 <= area.max.z
}


pub fn area_touches_block(area: &Area, block_pos: Vec3) -> bool {
	let corner = block_pos * 16;
	area.min.x <= corner.x + 15 && corner.x <= area.max.x
		&& area.min.y <= corner.y + 15 && corner.y <= area.max.y
		&& area.min.z <= corner.z + 15 && corner.z <= area.max.z
}


pub fn area_abs_block_overlap(area: &Area, block_pos: Vec3) -> Option<Area> {
	let block_min = block_pos * 16;
	let block_max = block_min + 15;
	let node_min = Vec3 {
		x: max(area.min.x, block_min.x),
		y: max(area.min.y, block_min.y),
		z: max(area.min.z, block_min.z)
	};
	let node_max = Vec3 {
		x: min(area.max.x, block_max.x),
		y: min(area.max.y, block_max.y),
		z: min(area.max.z, block_max.z)
	};

	if node_min.x <= node_max.x
		&& node_min.y <= node_max.y
		&& node_min.z <= node_max.z
	{
		Some(Area {min: node_min, max: node_max})
	} else {
		None
	}
}


pub fn area_rel_block_overlap(area: &Area, block_pos: Vec3) -> Option<Area> {
	let corner = block_pos * 16;
	let rel_min = area.min - corner;
	let rel_max = area.max - corner;
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
		Some(Area {min: node_min, max: node_max})
	} else {
		None
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_area_containment() {
		let area = Area::new(Vec3::new(-1, -32, 16), Vec3::new(30, -17, 54));
		let test_blocks = vec![
			// Fully contained
			(Vec3::new(0, -2, 1), true, true),
			(Vec3::new(0, -2, 2), true, true),
			// Partially contained
			(Vec3::new(-1, -2, 1), true, false),
			(Vec3::new(-1, -2, 2), true, false),
			(Vec3::new(-1, -2, 3), true, false),
			(Vec3::new(0, -2, 3), true, false),
			(Vec3::new(1, -2, 3), true, false),
			(Vec3::new(1, -2, 3), true, false),
			(Vec3::new(1, -2, 2), true, false),
			(Vec3::new(1, -2, 1), true, false),
			// Not contained
			(Vec3::new(-1, -2, 0), false, false),
			(Vec3::new(0, -2, 0), false, false),
			(Vec3::new(1, -2, 0), false, false),
			(Vec3::new(2, -2, 0), false, false),
			(Vec3::new(2, -2, 1), false, false),
			(Vec3::new(2, -2, 2), false, false),
			(Vec3::new(2, -2, 3), false, false),
		];

		for (pos, touches, contains) in test_blocks {
			assert_eq!(area_touches_block(&area, pos), touches);
			assert_eq!(area_contains_block(&area, pos), contains);
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
			assert_eq!(area_rel_block_overlap(&area, pair.0), pair.1);
		}
	}
}
