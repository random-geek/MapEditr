use super::Command;

use crate::spatial::{Vec3, area_rel_block_overlap,
	area_abs_block_overlap};
use crate::map_block::{MapBlock, NodeMetadataList};
use crate::block_utils::{merge_blocks, merge_metadata, clean_name_id_map};
use crate::instance::{ArgType, InstBundle};
use crate::utils::query_keys;
use crate::time_keeper::TimeKeeper;


// TODO: This and overlay--cache mapblocks in deserialized form.


fn clone(inst: &mut InstBundle) {
	let src_area = inst.args.area.unwrap();
	let offset = inst.args.offset.unwrap();
	let dst_area = src_area + offset;
	let mut keys = query_keys(&mut inst.db, &inst.status,
		Vec::new(), Some(dst_area), false, true);

	// Sort blocks according to offset such that we don't read blocks that
	// have already been written.
	let sort_dir = offset.map(|v| if v > 0 { -1 } else { 1 });
	// Subtract one from inverted axes to keep values from overflowing.
	let sort_offset = sort_dir.map(|v| if v == -1 { -1 } else { 0 });

	keys.sort_unstable_by_key(|k| {
		(Vec3::from_block_key(*k) * sort_dir + sort_offset).to_block_key()
	});

	inst.status.begin_editing();

	let mut tk = TimeKeeper::new();
	for key in keys {
		inst.status.inc_done();

		let dst_data = inst.db.get_block(key).unwrap();
		// TODO: is_valid_generated
		let mut dst_block = MapBlock::deserialize(&dst_data).unwrap();
		let mut dst_meta = NodeMetadataList::deserialize(
			dst_block.metadata.get_ref()).unwrap();

		let dst_pos = Vec3::from_block_key(key);
		let dst_part_abs = area_abs_block_overlap(&dst_area, dst_pos)
			.unwrap();
		let src_part_abs = dst_part_abs - offset;
		let src_blocks_needed = src_part_abs.to_touching_block_area();

		for src_pos in src_blocks_needed.iterate() {
			if !src_pos.is_valid_block_pos() {
				continue;
			}
			let src_data = inst.db.get_block(src_pos.to_block_key()).unwrap();
			let src_block = MapBlock::deserialize(&src_data).unwrap();
			let src_meta = NodeMetadataList::deserialize(
				&src_block.metadata.get_ref()).unwrap();

			let src_frag_abs = area_abs_block_overlap(&src_part_abs, src_pos)
				.unwrap();
			let src_frag_rel = src_frag_abs - src_pos * 16;
			let dst_frag_rel = area_rel_block_overlap(
				&(src_frag_abs + offset), dst_pos).unwrap();

			{
				let _t = tk.get_timer("merge");
				merge_blocks(&src_block, &mut dst_block,
					src_frag_rel, dst_frag_rel);
			}
			{
				let _t = tk.get_timer("merge_meta");
				merge_metadata(&src_meta, &mut dst_meta,
					src_frag_rel, dst_frag_rel);
			}
		}

		{
			let _t = tk.get_timer("name-ID map cleanup");
			clean_name_id_map(&mut dst_block);
		}

		*dst_block.metadata.get_mut() = dst_meta.serialize(dst_block.version);
		inst.db.set_block(key, &dst_block.serialize()).unwrap();
	}

	// tk.print();
	inst.status.end_editing();
}


pub fn get_command() -> Command {
	Command {
		func: clone,
		verify_args: None,
		args: vec![
			(ArgType::Area(true), "Area to clone"),
			(ArgType::Offset(true), "Vector to shift nodes by")
		],
		help: "Clone a given area to a new location."
	}
}
