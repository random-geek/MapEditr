use std::collections::BTreeMap;

use crate::map_block::{MapBlock, NodeMetadataList, NameIdMap};
use crate::spatial::{Vec3, Area};


fn block_parts_valid(a: &Area, b: &Area) -> bool {
	fn part_valid(a: &Area) -> bool {
		a.min.x >= 0 && a.min.y >= 0 && a.min.z >= 0
		&& a.max.x < 16 && a.max.y < 16 && a.max.z < 16
	}
	part_valid(a) && part_valid(b) && a.max - a.min == b.max - b.min
}


/// Copy an area of nodes from one mapblock to another.
///
/// Will not remove duplicate/unused name IDs.
pub fn merge_blocks(
	src_block: &MapBlock,
	dst_block: &mut MapBlock,
	src_area: Area,
	dst_area: Area
) {
	assert!(block_parts_valid(&src_area, &dst_area));

	let src_nd = &src_block.node_data;
	let dst_nd = &mut dst_block.node_data;
	let offset = dst_area.min - src_area.min;
	// Warning: diff can be negative!
	let diff = offset.x + offset.y * 16 + offset.z * 256;

	// Copy name-ID mappings
	let nimap_diff = dst_block.nimap.get_max_id().unwrap() + 1;
	for (id, name) in &src_block.nimap.0 {
		dst_block.nimap.0.insert(id + nimap_diff, name.to_vec());
	}

	// Copy node IDs
	for z in src_area.min.z ..= src_area.max.z {
		for y in src_area.min.y ..= src_area.max.y {
			for x in src_area.min.x ..= src_area.max.x {
				let idx = x + y * 16 + z * 256;
				dst_nd.nodes[(idx + diff) as usize] =
					src_nd.nodes[idx as usize] + nimap_diff;
			}
		}
	}

	// Copy param1 and param2
	for z in src_area.min.z ..= src_area.max.z {
		for y in src_area.min.y ..= src_area.max.y {
			let row_start = y * 16 + z * 256;
			let start = row_start + src_area.min.x;
			let end = row_start + src_area.max.x;

			dst_nd.param1[(start + diff) as usize ..= (end + diff) as usize]
				.clone_from_slice(
					&src_nd.param1[start as usize ..= end as usize]
				);
			dst_nd.param2[(start + diff) as usize ..= (end + diff) as usize]
				.clone_from_slice(
					&src_nd.param2[start as usize ..= end as usize]
				);
		}
	}
}


/// Copy an area of node metadata from one mapblock to another.
pub fn merge_metadata(
	src_meta: &NodeMetadataList,
	dst_meta: &mut NodeMetadataList,
	src_area: Area,
	dst_area: Area
) {
	assert!(block_parts_valid(&src_area, &dst_area));

	let offset = dst_area.min - src_area.min;
	// Warning: diff can be negative!
	let diff = offset.x + offset.y * 16 + offset.z * 256;

	// Delete any existing metadata in the destination area.
	let mut to_delete = Vec::with_capacity(dst_meta.len());
	for (&idx, _) in dst_meta.iter() {
		let pos = Vec3::from_u16_key(idx);
		if dst_area.contains(pos) {
			to_delete.push(idx);
		}
	}
	for idx in &to_delete {
		dst_meta.remove(idx);
	}

	// Copy new metadata
	for (&idx, meta) in src_meta {
		let pos = Vec3::from_u16_key(idx);
		if src_area.contains(pos) {
			dst_meta.insert((idx as i32 + diff) as u16, meta.clone());
		}
	}
}


/// Culls duplicate and unused IDs from the name-ID map and node data.
pub fn clean_name_id_map(block: &mut MapBlock) {
	let id_count = (block.nimap.get_max_id().unwrap() + 1) as usize;

	// Determine which IDs are used.
	let mut used = vec![false; id_count];
	for id in &block.node_data.nodes {
		used[*id as usize] = true;
	}

	// Rebuild the name-ID map.
	let mut new_nimap = NameIdMap(BTreeMap::new());
	let mut map = vec![0u16; id_count]; // map[old_node_id] == new_node_id
	for (&id, name) in &block.nimap.0 {
		// Skip unused IDs.
		if !used[id as usize] {
			continue;
		}

		if let Some(first_id) = new_nimap.get_id(&name) {
			// Name is already in the map; map old, duplicate ID to the
			// existing ID.
			map[id as usize] = first_id as u16;
		} else {
			// Name is not yet in the map; assign it to the next ID.
			new_nimap.0.insert(new_nimap.0.len() as u16, name.clone());
			// Map old ID to newly-inserted ID.
			map[id as usize] = new_nimap.0.len() as u16 - 1;
		}
	}
	block.nimap = new_nimap;

	// Re-assign node IDs.
	for id in &mut block.node_data.nodes {
		*id = map[*id as usize];
	}
}
