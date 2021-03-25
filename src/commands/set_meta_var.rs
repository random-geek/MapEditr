use super::Command;

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList, NodeMetadataListExt};
use crate::utils::{query_keys, to_bytes, fmt_big_num};


fn set_meta_var(inst: &mut InstBundle) {
	// TODO: Bytes input, create/delete variables
	let key = to_bytes(inst.args.key.as_ref().unwrap());
	let value = to_bytes(inst.args.value.as_ref().unwrap());
	let nodes: Vec<_> = inst.args.nodes.iter().map(to_bytes).collect();

	let keys = query_keys(&mut inst.db, &mut inst.status,
		&nodes, inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count: u64 = 0;

	for block_key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(block_key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		let node_data = block.node_data.get_ref();
		let node_ids: Vec<_> = nodes.iter()
			.filter_map(|n| block.nimap.get_id(n)).collect();
		if !nodes.is_empty() && node_ids.is_empty() {
			continue; // Block doesn't contain any of the required nodes.
		}

		let mut meta = unwrap_or!(
			NodeMetadataList::deserialize(block.metadata.get_ref()),
			{ inst.status.inc_failed(); continue; });

		let block_corner = Vec3::from_block_key(block_key) * 16;
		let mut modified = false;

		for (&idx, data) in &mut meta {
			let pos = Vec3::from_u16_key(idx);
			let abs_pos = pos + block_corner;

			if let Some(a) = inst.args.area {
				if a.contains(abs_pos) == inst.args.invert {
					continue;
				}
			}
			if !node_ids.is_empty()
				&& !node_ids.contains(&node_data.nodes[idx as usize])
			{
				continue;
			}

			if let Some(val) = data.vars.get_mut(&key) {
				val.0 = value.clone();
				modified = true;
				count += 1;
			}
		}

		if modified {
			*block.metadata.get_mut() = meta.serialize(block.version);
			inst.db.set_block(block_key, &block.serialize()).unwrap();
		}
	}

	inst.status.end_editing();
	inst.status.log_info(
		format!("Set metadata variable of {} nodes.", fmt_big_num(count)));
}


pub fn get_command() -> Command {
	Command {
		func: set_meta_var,
		verify_args: None,
		args: vec![
			(ArgType::Key, "Name of key to set in metadata"),
			(ArgType::Value, "Value to set in metadata"),
			(ArgType::Area(false),
				"Optional area in which to modify node metadata"),
			(ArgType::Invert, "Modify node metadata outside the given area."),
			(ArgType::Nodes,
				"Names of one or more nodes to modify. If not specified, all \
				nodes with the specified variable will be modified.")
		],
		help: "Set a variable in node metadata."
	}
}
