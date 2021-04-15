use super::Command;

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::MapBlock;
use crate::utils::{query_keys, to_bytes, to_slice, fmt_big_num};


fn delete_timers(inst: &mut InstBundle) {
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

		let node_id = node.as_deref().and_then(|n| block.nimap.get_id(n));
		if node.is_some() && node_id.is_none() {
			continue; // Block doesn't contain the required node.
		}
		let node_data = block.node_data.get_ref();

		let block_corner = Vec3::from_block_key(key) * 16;
		let mut modified = false;

		for i in (0..block.node_timers.len()).rev() {
			let pos_idx = block.node_timers[i].pos;
			let pos = Vec3::from_u16_key(pos_idx);
			let abs_pos = pos + block_corner;

			if let Some(a) = inst.args.area {
				if a.contains(abs_pos) == inst.args.invert {
					continue;
				}
			}
			if let Some(id) = node_id {
				if node_data.nodes[pos_idx as usize] != id {
					continue;
				}
			}

			block.node_timers.remove(i);
			count += 1;
			modified = true;
		}

		if modified {
			inst.db.set_block(key, &block.serialize()).unwrap();
		}
	}

	inst.status.end_editing();
	inst.status.log_info(
		format!("Deleted {} node timers.", fmt_big_num(count)));
}


pub fn get_command() -> Command {
	Command {
		func: delete_timers,
		verify_args: None,
		args: vec![
			(ArgType::Node(false), "Name of node to delete node timers from"),
			(ArgType::Area(false), "Area in which to delete node timers"),
			(ArgType::Invert, "Delete node timers *outside* the given area."),
		],
		help: "Delete node timers of certain nodes."
	}
}
