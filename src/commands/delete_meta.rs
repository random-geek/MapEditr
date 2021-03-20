use super::Command;

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList, NodeMetadataListExt};
use crate::utils::{query_keys, to_bytes, to_slice, fmt_big_num};


fn delete_metadata(inst: &mut InstBundle) {
	let node = inst.args.node.as_ref().map(to_bytes);

	let keys = query_keys(&mut inst.db, &mut inst.status,
		to_slice(&node), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count: u64 = 0;

	for key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		let node_data = block.node_data.get_ref();
		let node_id = node.as_deref().and_then(|n| block.nimap.get_id(n));
		if node.is_some() && node_id.is_none() {
			continue; // Block doesn't contain the required node.
		}

		let mut meta = unwrap_or!(
			NodeMetadataList::deserialize(block.metadata.get_ref()),
			{ inst.status.inc_failed(); continue; });

		let block_corner = Vec3::from_block_key(key) * 16;
		let mut to_delete = Vec::with_capacity(meta.len());

		for (&idx, _) in &meta {
			let abs_pos = Vec3::from_u16_key(idx) + block_corner;

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

			to_delete.push(idx);
		}

		if !to_delete.is_empty() {
			for idx in &to_delete {
				meta.remove(idx);
			}
			count += to_delete.len() as u64;
			*block.metadata.get_mut() = meta.serialize(block.version);
			inst.db.set_block(key, &block.serialize()).unwrap();
		}
	}

	inst.status.end_editing();
	inst.status.log_info(
		format!("Deleted metadata from {} nodes.", fmt_big_num(count)));
}


pub fn get_command() -> Command {
	Command {
		func: delete_metadata,
		verify_args: None,
		args: vec![
			(ArgType::Area(false), "Area in which to delete metadata"),
			(ArgType::Invert, "Delete all metadata outside the given area."),
			(ArgType::Node(false),
				"Node to delete metadata from. If not specified, all metadata \
				will be deleted.")
		],
		help: "Delete node metadata."
	}
}
