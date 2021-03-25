use super::Command;

use crate::unwrap_or;
use crate::spatial::{Vec3, Area, InverseBlockIterator};
use crate::instance::{ArgType, InstBundle};
use crate::map_block::MapBlock;
use crate::block_utils::clean_name_id_map;
use crate::utils::{query_keys, to_bytes, fmt_big_num};


fn fill_area(block: &mut MapBlock, id: u16, area: Area, invert: bool) {
	let nd = block.node_data.get_mut();

	if invert {
		for i in InverseBlockIterator::new(area) {
			nd.nodes[i] = id;
		}
	} else {
		for z in area.min.z ..= area.max.z {
			let z_start = z * 256;
			for y in area.min.y ..= area.max.y {
				let zy_start = z_start + y * 16;
				for x in area.min.x ..= area.max.x {
					nd.nodes[(zy_start + x) as usize] = id;
				}
			}
		}
	}
}


fn fill(inst: &mut InstBundle) {
	let area = inst.args.area.unwrap();
	let node = to_bytes(inst.args.new_node.as_ref().unwrap());

	let keys = query_keys(&mut inst.db, &mut inst.status,
		&[], Some(area), inst.args.invert, true);

	inst.status.begin_editing();

	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();

		let pos = Vec3::from_block_key(key);
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		if area.contains_block(pos) != area.touches_block(pos) {
			// Fill part of block
			let block_part = area.rel_block_overlap(pos).unwrap();
			let fill_id = block.nimap.get_id(&node).unwrap_or_else(|| {
				let next = block.nimap.get_max_id().unwrap() + 1;
				block.nimap.0.insert(next, node.to_vec());
				next
			});
			fill_area(&mut block, fill_id, block_part, inst.args.invert);
			clean_name_id_map(&mut block);
			count += block_part.volume();
		} else { // Fill entire block
			let nd = block.node_data.get_mut();
			nd.nodes.fill(0);
			block.nimap.0.clear();
			block.nimap.0.insert(0, node.to_vec());
			count += nd.nodes.len() as u64;
		}

		inst.db.set_block(key, &block.serialize()).unwrap();
	}

	inst.status.end_editing();
	inst.status.log_info(format!("{} nodes filled.", fmt_big_num(count)));
}


pub fn get_command() -> Command {
	Command {
		func: fill,
		verify_args: None,
		args: vec![
			(ArgType::Area(true), "Area to fill"),
			(ArgType::Invert,
				"Fill all generated nodes *outside* the given area."),
			(ArgType::NewNode, "Name of node to fill the area with"),
		],
		help: "Fill the entire area with one node."
	}
}
