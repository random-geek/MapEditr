use super::Command;

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList};
use crate::utils::{query_keys, fmt_big_num};


fn set_meta_var(inst: &mut InstBundle) {
	let key = inst.args.key.as_ref().unwrap().as_bytes().to_owned();
	let value = inst.args.value.as_ref().unwrap().as_bytes().to_owned();
	let node = inst.args.node.as_ref().map(|s| s.as_bytes().to_owned());

	let keys = query_keys(&mut inst.db, &mut inst.status,
		node.as_deref(), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count: u64 = 0;

	for block_key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(block_key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data), continue);

		let node_data = block.node_data.get_ref();
		let node_id = node.as_deref().and_then(|n| block.nimap.get_id(n));
		if node.is_some() && node_id.is_none() {
			continue; // Block doesn't contain the required node.
		}

		let mut meta = unwrap_or!(
			NodeMetadataList::deserialize(block.metadata.get_ref()), continue);

		let block_corner = Vec3::from_block_key(block_key) * 16;
		let mut modified = false;

		for (&idx, data) in &mut meta.list {
			let pos = Vec3::from_u16_key(idx);
			let abs_pos = pos + block_corner;

			if let Some(a) = inst.args.area {
				if a.contains(abs_pos) == inst.args.invert {
					continue;
				}
			}
			if let Some(id) = node_id {
				if node_data.nodes[idx as usize] != id {
					continue;
				}
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
			(ArgType::Area(false), "Area in which to modify node metadata"),
			(ArgType::Invert, "Modify node metadata outside the given area."),
			(ArgType::Node(false),
				"Node to modify metadata in. If not specified, all relevant \
				metadata will be modified.")
		],
		help: "Set a variable in node metadata."
	}
}
