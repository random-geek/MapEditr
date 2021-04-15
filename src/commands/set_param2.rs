use super::{Command, ArgResult};

use crate::unwrap_or;
use crate::spatial::{Vec3, Area, InverseBlockIterator};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::MapBlock;
use crate::utils::{query_keys, to_bytes, to_slice, fmt_big_num};


fn set_param2_partial(block: &mut MapBlock, area: Area, invert: bool,
	node_id: Option<u16>, val: u8) -> u64
{
	let nd = block.node_data.get_mut();
	let mut count = 0;

	if invert {
		if let Some(id) = node_id {
			for idx in InverseBlockIterator::new(area) {
				if nd.nodes[idx] == id {
					nd.param2[idx] = val;
					count += 1;
				}
			}
		} else {
			for idx in InverseBlockIterator::new(area) {
				nd.param2[idx] = val;
			}
			count += 4096 - area.volume();
		}
	} else {
		let no_node = node_id.is_none();
		let id = node_id.unwrap_or(0);

		for z in area.min.z ..= area.max.z {
			let z_start = z * 256;
			for y in area.min.y ..= area.max.y {
				let zy_start = z_start + y * 16;
				for x in area.min.x ..= area.max.x {
					let i = (zy_start + x) as usize;
					if no_node || nd.nodes[i] == id {
						nd.param2[i] = val;
						count += 1;
					}
				}
			}
		}
	}

	count
}


fn set_param2(inst: &mut InstBundle) {
	let param2_val = inst.args.param2.unwrap();
	let node = inst.args.node.as_ref().map(to_bytes);

	let keys = query_keys(&mut inst.db, &mut inst.status,
		to_slice(&node), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();

	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();

		let pos = Vec3::from_block_key(key);
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		let node_id = node.as_ref().and_then(|n| block.nimap.get_id(n));
		if inst.args.node.is_some() && node_id.is_none() {
			// Node not found in this mapblock.
			continue;
		}

		let nd = block.node_data.get_mut();
		if let Some(area) = inst.args.area
			.filter(|a| a.contains_block(pos) != a.touches_block(pos))
		{ // Modify part of block
			let block_part = area.rel_block_overlap(pos).unwrap();
			count += set_param2_partial(&mut block,
				block_part, inst.args.invert, node_id, param2_val);
		} else { // Modify whole block
			if let Some(nid) = node_id {
				for i in 0 .. nd.param2.len() {
					if nd.nodes[i] == nid {
						nd.param2[i] = param2_val;
						count += 1;
					}
				}
			} else {
				nd.param2.fill(param2_val);
				count += nd.param2.len() as u64;
			}
		}

		inst.db.set_block(key, &block.serialize()).unwrap();
	}

	inst.status.end_editing();
	inst.status.log_info(format!("Set param2 of {} nodes.",
		fmt_big_num(count)));
}


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.area.is_none() && args.node.is_none() {
		return ArgResult::error("An area and/or node is required.");
	}

	ArgResult::Ok
}


pub fn get_command() -> Command {
	Command {
		func: set_param2,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Node(false), "Name of node to modify"),
			(ArgType::Area(false), "Area in which to set param2 values"),
			(ArgType::Invert, "Set param2 values *outside* the given area."),
			(ArgType::Param2, "New param2 value, between 0 and 255"),
		],
		help: "Set param2 values of certain nodes."
	}
}
