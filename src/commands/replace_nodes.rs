use super::Command;

use crate::spatial::{Vec3, Area, area_contains_block, area_touches_block,
	area_rel_block_overlap};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::MapBlock;
use crate::utils::query_keys;
use crate::time_keeper::TimeKeeper;
use crate::utils::fmt_big_num;


fn do_replace(
	block: &mut MapBlock,
	key: i64,
	search_id: u16,
	new_node: &[u8],
	area: Option<Area>,
	invert: bool,
	tk: &mut TimeKeeper
) -> u64
{
	let block_pos = Vec3::from_block_key(key);
	let mut count = 0;

	// Replace nodes in a portion of a map block.
	if area.is_some() && area_contains_block(&area.unwrap(), block_pos) !=
		area_touches_block(&area.unwrap(), block_pos)
	{
		let _t = tk.get_timer("replace (partial block)");
		let node_area = area_rel_block_overlap(&area.unwrap(), block_pos)
			.unwrap();

		let mut new_replace_id = false;
		let replace_id = block.nimap.get_id(new_node)
			.unwrap_or_else(|| {
				new_replace_id = true;
				block.nimap.get_max_id().unwrap() + 1
			});

		let mut idx = 0;
		let mut old_node_present = false;
		let mut new_node_present = false;

		let nd = block.node_data.get_mut();
		for z in 0 .. 16 {
			for y in 0 .. 16 {
				for x in 0 .. 16 {
					if nd.nodes[idx] == search_id
						&& node_area.contains(Vec3 {x, y, z}) != invert
					{
						nd.nodes[idx] = replace_id;
						new_node_present = true;
						count += 1;
					}

					if nd.nodes[idx] == search_id {
						old_node_present = true;
					}
					idx += 1;
				}
			}
		}

		// Replacement node not yet in name-ID map; insert it.
		if new_replace_id && new_node_present {
			block.nimap.insert(replace_id, new_node);
		}

		// Search node was completely eliminated; shift IDs down.
		if !old_node_present {
			for i in 0 .. nd.nodes.len() {
				if nd.nodes[i] > search_id {
					nd.nodes[i] -= 1;
				}
			}
			block.nimap.remove(search_id);
		}
	}
	// Replace nodes in whole map block.
	else {
		// Block already contains replacement node, beware!
		if let Some(mut replace_id) = block.nimap.get_id(new_node) {
			let _t = tk.get_timer("replace (non-unique replacement)");
			// Delete unused ID from name-ID map and shift IDs down.
			block.nimap.remove(search_id);
			// Shift replacement ID, if necessary.
			replace_id -= (replace_id > search_id) as u16;

			// Map old node IDs to new node IDs.
			let nd = block.node_data.get_mut();
			for id in &mut nd.nodes {
				*id = if *id == search_id {
					count += 1;
					replace_id
				} else {
					*id - (*id > search_id) as u16
				};
			}
		}
		// Block does not contain replacement node.
		// Simply replace the node name in the name-ID map.
		else {
			let _t = tk.get_timer("replace (unique replacement)");
			let nd = block.node_data.get_ref();
			for id in &nd.nodes {
				count += (*id == search_id) as u64;
			}
			block.nimap.insert(search_id, new_node);
		}
	}
	count
}


fn replace_nodes(inst: &mut InstBundle) {
	let node = inst.args.node.as_ref().unwrap().as_bytes();
	let new_node = inst.args.new_node.as_ref().unwrap().as_bytes();
	let keys = query_keys(&mut inst.db, &inst.status,
		Some(node), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count = 0;

	let mut tk = TimeKeeper::new();
	for key in keys {
		let data = inst.db.get_block(key).unwrap();

		let mut block = {
			let _t = tk.get_timer("decode");
			MapBlock::deserialize(&data).unwrap()
		};

		if let Some(search_id) = block.nimap.get_id(&node) {
			count += do_replace(&mut block, key, search_id, &new_node,
				inst.args.area, inst.args.invert, &mut tk);
			let new_data = {
				let _t = tk.get_timer("encode");
				block.serialize()
			};
			inst.db.set_block(key, &new_data).unwrap();
		}

		inst.status.inc_done();
	}

	// tk.print();
	inst.status.end_editing();
	inst.status.log_info(
		format!("{} nodes replaced.", fmt_big_num(count)).as_str());
}


fn verify_args(args: &InstArgs) -> anyhow::Result<()> {
	anyhow::ensure!(args.node != args.new_node,
		"node and new_node must be different.");
	Ok(())
}


pub fn get_command() -> Command {
	Command {
		func: replace_nodes,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Node(true), "Name of node to replace"),
			(ArgType::NewNode, "Name of node to replace with"),
			(ArgType::Area(false), "Area in which to replace nodes"),
			(ArgType::Invert, "Replace nodes outside the given area")
		],
		help: "Replace all of one node with another node."
	}
}
