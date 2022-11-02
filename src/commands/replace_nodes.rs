use super::{Command, ArgResult};

use crate::unwrap_or;
use crate::spatial::{Vec3, Area, InverseBlockIterator};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::MapBlock;
use crate::utils::{query_keys, to_bytes, fmt_big_num};


fn do_replace(
	block: &mut MapBlock,
	key: i64,
	old_id: u16,
	new_node: &[u8],
	area: Option<Area>,
	invert: bool
) -> u64
{
	let nodes = &mut block.node_data.nodes;
	let block_pos = Vec3::from_block_key(key);
	let mut replaced = 0;

	// Replace nodes in a portion of the mapblock.
	if area
		.filter(|a| a.contains_block(block_pos) != a.touches_block(block_pos))
		.is_some()
	{
		let node_area = area.unwrap().rel_block_overlap(block_pos).unwrap();

		let (new_id, new_id_needed) = match block.nimap.get_id(new_node) {
			Some(id) => (id, false),
			None => (block.nimap.get_max_id().unwrap() + 1, true)
		};

		if invert {
			for idx in InverseBlockIterator::new(node_area) {
				if nodes[idx] == old_id {
					nodes[idx] = new_id;
					replaced += 1;
				}
			}
		} else {
			for pos in &node_area {
				let idx = (pos.x + 16 * (pos.y + 16 * pos.z)) as usize;
				if nodes[idx] == old_id {
					nodes[idx] = new_id;
					replaced += 1;
				}
			}
		}

		// If replacement ID is not in the name-ID map but was used, add it.
		if new_id_needed && replaced > 0 {
			block.nimap.0.insert(new_id, new_node.to_vec());
		}

		// If all instances of the old ID were replaced, remove the old ID.
		if !nodes.contains(&old_id) {
			for node in nodes {
				*node -= (*node > old_id) as u16;
			}
			block.nimap.remove_shift(old_id);
		}
	}
	// Replace nodes in whole mapblock.
	else {
		// Block already contains replacement node, beware!
		if let Some(mut new_id) = block.nimap.get_id(new_node) {
			// Delete unused ID from name-ID map and shift IDs down.
			block.nimap.remove_shift(old_id);
			// Shift replacement ID, if necessary.
			new_id -= (new_id > old_id) as u16;

			// Map old node IDs to new node IDs.
			for id in nodes {
				*id = if *id == old_id {
					replaced += 1;
					new_id
				} else {
					*id - (*id > old_id) as u16
				};
			}
		}
		// Block does not contain replacement node.
		// Simply replace the node name in the name-ID map.
		else {
			for id in nodes {
				replaced += (*id == old_id) as u64;
			}
			block.nimap.0.insert(old_id, new_node.to_vec());
		}
	}
	replaced
}


fn replace_nodes(inst: &mut InstBundle) {
	let old_node = to_bytes(inst.args.node.as_ref().unwrap());
	let new_node = to_bytes(inst.args.new_node.as_ref().unwrap());
	let keys = query_keys(&mut inst.db, &inst.status,
		std::slice::from_ref(&old_node),
		inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count = 0;

	for key in keys {
		let data = inst.db.get_block(key).unwrap();

		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		if let Some(old_id) = block.nimap.get_id(&old_node) {
			count += do_replace(&mut block, key, old_id, &new_node,
				inst.args.area, inst.args.invert);
			let new_data = block.serialize();
			inst.db.set_block(key, &new_data).unwrap();
		}

		inst.status.inc_done();
	}

	inst.status.end_editing();
	inst.status.log_info(format!("{} nodes replaced.", fmt_big_num(count)));
}


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.node == args.new_node {
		return ArgResult::error("node and new_node must be different.");
	}

	ArgResult::Ok
}


pub fn get_command() -> Command {
	Command {
		func: replace_nodes,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Node(true), "Name of node to replace"),
			(ArgType::NewNode, "Name of node to replace with"),
			(ArgType::Area(false), "Area in which to replace nodes"),
			(ArgType::Invert, "Replace nodes *outside* the given area.")
		],
		help: "Replace one node with another node."
	}
}
