use super::Command;

use crate::spatial::{Vec3, Area, area_rel_block_overlap, area_contains_block};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::{MapBlock};
use crate::utils::{query_keys, to_bytes, to_slice, fmt_big_num};


fn set_in_area_node(block: &mut MapBlock, area: Area, id: u16, val: u8) -> u64
{
	let nd = block.node_data.get_mut();
	let mut count = 0;
	for z in area.min.z ..= area.max.z {
		let z_start = z * 256;
		for y in area.min.y ..= area.max.y {
			let zy_start = z_start + y * 16;
			for x in area.min.x ..= area.max.x {
				let i = (zy_start + x) as usize;
				if nd.nodes[i] == id {
					nd.param2[i] = val;
					count += 1;
				}
			}
		}
	}
	count
}


fn set_in_area(block: &mut MapBlock, area: Area, val: u8) {
	let nd = block.node_data.get_mut();
	for z in area.min.z ..= area.max.z {
		let z_start = z * 256;
		for y in area.min.y ..= area.max.y {
			let zy_start = z_start + y * 16;
			for x in area.min.x ..= area.max.x {
				nd.param2[(zy_start + x) as usize] = val;
			}
		}
	}
}


fn set_param2(inst: &mut InstBundle) {
	let param2_val = inst.args.param2_val.unwrap();
	let node = inst.args.node.as_ref().map(to_bytes);

	let keys = query_keys(&mut inst.db, &mut inst.status,
		to_slice(&node), inst.args.area, false, true);

	inst.status.begin_editing();

	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();

		let pos = Vec3::from_block_key(key);
		let data = inst.db.get_block(key).unwrap();
		let mut block = MapBlock::deserialize(&data).unwrap();

		let node_id = node.as_ref().and_then(|n| block.nimap.get_id(n));
		if inst.args.node.is_some() && node_id.is_none() {
			// Node not found in this map block.
			continue;
		}

		let nd = block.node_data.get_mut();
		if let Some(area) = inst.args.area
			.filter(|a| !area_contains_block(&a, pos))
		{ // Modify part of block
			let overlap = area_rel_block_overlap(&area, pos).unwrap();
			if let Some(nid) = node_id {
				count +=
					set_in_area_node(&mut block, overlap, nid, param2_val);
			} else {
				set_in_area(&mut block, overlap, param2_val);
				count += overlap.volume();
			}
		} else { // Modify whole block
			if let Some(nid) = node_id {
				for i in 0 .. nd.param2.len() {
					if nd.nodes[i] == nid {
						nd.param2[i] = param2_val;
						count += 1;
					}
				}
			} else {
				for x in &mut nd.param2 {
					*x = param2_val;
				}
				count += nd.param2.len() as u64;
			}
		}

		inst.db.set_block(key, &block.serialize()).unwrap();
	}

	inst.status.end_editing();
	inst.status.log_info(
		format!("{} nodes set.", fmt_big_num(count)).as_str());
}


fn verify_args(args: &InstArgs) -> anyhow::Result<()> {
	anyhow::ensure!(args.area.is_some() || args.node.is_some(),
		"An area and/or node must be provided.");
	Ok(())
}


pub fn get_command() -> Command {
	Command {
		func: set_param2,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Area(false), "Area in which to set param2 values"),
			(ArgType::Node(false), "Node to set param2 values of"),
			(ArgType::Param2Val, "New param2 value")
		],
		help: "Set param2 values of an area or node."
	}
}
