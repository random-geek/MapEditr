use super::Command;

use crate::spatial::{Vec3, Area, area_rel_block_overlap,
	area_abs_block_overlap, area_contains_block, area_touches_block};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList, is_valid_generated};
use crate::block_utils::{merge_blocks, merge_metadata, clean_name_id_map};
use crate::utils::query_keys;


fn verify_args(args: &InstArgs) -> anyhow::Result<()> {
	let offset_if_nonzero =
		args.offset.filter(|&off| off != Vec3::new(0, 0, 0));
	if args.invert && offset_if_nonzero.is_some() {
		anyhow::bail!("Inverted selections cannot be offset.");
	}
	Ok(())
}


/// Overlay without offsetting anything.
///
/// Possible argument configurations:
/// - No arguments (copy everything)
/// - Area
/// - Area + Invert
#[inline]
fn overlay_no_offset(inst: &mut InstBundle) {
	let mut idb = inst.idb.as_mut().unwrap();
	let invert = inst.args.invert;

	// Get keys from input database.
	let keys = query_keys(&mut idb, &inst.status,
		&[], inst.args.area, invert, true);
	inst.status.begin_editing();

	for key in keys {
		inst.status.inc_done();

		if let Some(area) = inst.args.area {
			let pos = Vec3::from_block_key(key);

			if (!invert && area_contains_block(&area, pos))
				|| (invert && !area_touches_block(&area, pos))
			{ // If possible, copy whole map block.
				let data = idb.get_block(key).unwrap();
				if is_valid_generated(&data) {
					inst.db.set_block(key, &data).unwrap();
				}
			} else { // Copy part of map block
				let dst_data = match inst.db.get_block(key) {
					Ok(d) => if is_valid_generated(&d) {
						d
					} else {
						continue;
					},
					Err(_) => continue
				};
				let src_data = idb.get_block(key).unwrap();

				let mut src_block = MapBlock::deserialize(&src_data).unwrap();
				let mut dst_block = MapBlock::deserialize(&dst_data).unwrap();
				let mut src_meta = NodeMetadataList::deserialize(
					&src_block.metadata.get_ref()).unwrap();
				let mut dst_meta = NodeMetadataList::deserialize(
					&dst_block.metadata.get_ref()).unwrap();

				let block_part = area_rel_block_overlap(&area, pos).unwrap();
				if invert {
					// For inverted selections, reverse the order of the
					// overlay operations.
					merge_blocks(&dst_block, &mut src_block,
						block_part, block_part);
					merge_metadata(&dst_meta, &mut src_meta,
						block_part, block_part);
					clean_name_id_map(&mut src_block);
					inst.db.set_block(key, &src_block.serialize()).unwrap();
				} else {
					merge_blocks(&src_block, &mut dst_block,
						block_part, block_part);
					merge_metadata(&src_meta, &mut dst_meta,
						block_part, block_part);
					clean_name_id_map(&mut dst_block);
					inst.db.set_block(key, &dst_block.serialize()).unwrap();
				}
			}
		} else {
			// No area; copy whole map block.
			let data = idb.get_block(key).unwrap();
			if is_valid_generated(&data) {
				inst.db.set_block(key, &data).unwrap();
			}
		}
	}

	inst.status.end_editing();
}


/// Overlay with offset, with or without area.
#[inline]
fn overlay_with_offset(inst: &mut InstBundle) {
	let offset = inst.args.offset.unwrap();
	let src_area = inst.args.area;
	let dst_area = src_area.map(|a| a + offset);
	let idb = inst.idb.as_mut().unwrap();

	// Get keys from output database.
	let keys = query_keys(&mut inst.db, &inst.status,
		&[], dst_area, inst.args.invert, true);
	inst.status.begin_editing();

	for key in keys {
		inst.status.inc_done();

		let dst_pos = Vec3::from_block_key(key);
		let dst_data = inst.db.get_block(key).unwrap();
		if !is_valid_generated(&dst_data) {
			continue;
		}
		let mut dst_block = MapBlock::deserialize(&dst_data).unwrap();
		let mut dst_meta = NodeMetadataList::deserialize(
			dst_block.metadata.get_ref()).unwrap();

		let dst_part_abs = dst_area.map_or(
			Area::new(dst_pos * 16, dst_pos * 16 + 15),
			|ref a| area_abs_block_overlap(a, dst_pos).unwrap()
		);
		let src_part_abs = dst_part_abs - offset;
		let src_blocks_needed = src_part_abs.to_touching_block_area();

		for src_pos in src_blocks_needed.iterate() {
			if !src_pos.is_valid_block_pos() {
				continue;
			}
			let src_data = match idb.get_block(src_pos.to_block_key()) {
				Ok(d) => if is_valid_generated(&d) {
					d
				} else {
					continue
				},
				Err(_) => continue
			};
			let src_block = MapBlock::deserialize(&src_data).unwrap();
			let src_meta = NodeMetadataList::deserialize(
				src_block.metadata.get_ref()).unwrap();

			let src_frag_abs = area_abs_block_overlap(&src_part_abs, src_pos)
				.unwrap();
			let src_frag_rel = src_frag_abs - src_pos * 16;
			let dst_frag_rel = area_rel_block_overlap(
				&(src_frag_abs + offset), dst_pos).unwrap();

			merge_blocks(&src_block, &mut dst_block,
				src_frag_rel, dst_frag_rel);
			merge_metadata(&src_meta, &mut dst_meta,
				src_frag_rel, dst_frag_rel);
		}

		clean_name_id_map(&mut dst_block);
		*dst_block.metadata.get_mut() = dst_meta.serialize(dst_block.version);
		inst.db.set_block(key, &dst_block.serialize()).unwrap();
	}

	inst.status.end_editing();
}


fn overlay(inst: &mut InstBundle) {
	let offset = inst.args.offset.unwrap_or(Vec3::new(0, 0, 0));
	if offset == Vec3::new(0, 0, 0) {
		overlay_no_offset(inst);
	} else {
		overlay_with_offset(inst);
	}
}


pub fn get_command() -> Command {
	Command {
		func: overlay,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::InputMapPath, "Path to input map file"),
			(ArgType::Area(false), "Area to overlay"),
			(ArgType::Invert, "Overlay all nodes outside the given area"),
			(ArgType::Offset(false), "Vector to offset nodes by"),
		],
		help: "Copy part or all of one map into another."
	}
}
