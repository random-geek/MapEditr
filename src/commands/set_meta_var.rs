use super::{Command, ArgResult};

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::MapBlock;
use crate::utils::{query_keys, to_bytes, fmt_big_num};


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.value.is_none() && !args.delete {
		return ArgResult::error(
			"value is required unless deleting the variable.");
	} else if args.value.is_some() && args.delete {
		return ArgResult::error(
			"value cannot be used when deleting the variable.");
	} else if args.value == Some(String::new()) {
		return ArgResult::error("Metadata value cannot be empty.");
	}
	ArgResult::Ok
}


fn set_meta_var(inst: &mut InstBundle) {
	// TODO: Bytes input
	let key = to_bytes(inst.args.key.as_ref().unwrap());
	let value = to_bytes(inst.args.value.as_ref().unwrap_or(&String::new()));
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

		let node_ids: Vec<_> = nodes.iter()
			.filter_map(|n| block.nimap.get_id(n)).collect();
		if !nodes.is_empty() && node_ids.is_empty() {
			continue; // Block doesn't contain any of the required nodes.
		}

		let block_corner = Vec3::from_block_key(block_key) * 16;
		let mut modified = false;

		for (&idx, data) in &mut block.metadata {
			let pos = Vec3::from_u16_key(idx);

			if let Some(a) = inst.args.area {
				if a.contains(pos + block_corner) == inst.args.invert {
					continue;
				}
			}
			if !node_ids.is_empty()
				&& !node_ids.contains(&block.node_data.nodes[idx as usize])
			{
				continue;
			}

			if data.vars.contains_key(&key) {
				if inst.args.delete {
					// Note: serialize() will cull any newly empty metadata.
					data.vars.remove(&key);
				} else {
					data.vars.get_mut(&key).unwrap().0 = value.clone();
				}
				modified = true;
				count += 1;
			}
		}

		if modified {
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
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Key, "Name of variable to set/delete"),
			(ArgType::Value, "Value to set variable to, if setting a value"),
			(ArgType::Delete, "Delete the variable."),
			(ArgType::Nodes,
				"Names of one or more nodes to modify. If not specified, any \
				node with the given variable will be modified."),
			(ArgType::Area(false),
				"Area in which to modify node metadata"),
			(ArgType::Invert, "Modify node metadata *outside* the given area."),
		],
		help: "Set or delete a variable in node metadata of certain nodes."
	}
}
