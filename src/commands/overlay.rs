use super::{Command, ArgResult, BLOCK_CACHE_SIZE};

use crate::{unwrap_or, opt_unwrap_or};
use crate::spatial::{Vec3, Area, MAP_LIMIT};
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_database::MapDatabase;
use crate::map_block::{MapBlock, MapBlockError, is_valid_generated,
	NodeMetadataList, NodeMetadataListExt};
use crate::block_utils::{merge_blocks, merge_metadata, clean_name_id_map};
use crate::utils::{query_keys, CacheMap};


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.invert
		&& args.offset.filter(|&ofs| ofs != Vec3::new(0, 0, 0)).is_some()
	{
		return ArgResult::error("Inverted selections cannot be offset.");
	}

	let offset = args.offset.unwrap_or(Vec3::new(0, 0, 0));
	let map_area = Area::new(
		Vec3::new(-MAP_LIMIT, -MAP_LIMIT, -MAP_LIMIT),
		Vec3::new(MAP_LIMIT, MAP_LIMIT, MAP_LIMIT)
	);

	if map_area.intersection(args.area.unwrap_or(map_area) + offset)
		.is_none()
	{
		return ArgResult::error("Destination area is outside map bounds.");
	}

	ArgResult::Ok
}


/// Overlay without offsetting anything.
///
/// Possible argument configurations:
/// - No arguments (copy everything)
/// - Area
/// - Area + Invert
#[inline]
fn overlay_no_offset(inst: &mut InstBundle) {
	let db = &mut inst.db;
	let idb = inst.idb.as_mut().unwrap();
	let invert = inst.args.invert;

	// Get keys from input database.
	let keys = query_keys(idb, &inst.status,
		&[], inst.args.area, invert, true);
	inst.status.begin_editing();

	for key in keys {
		inst.status.inc_done();

		if let Some(area) = inst.args.area {
			let pos = Vec3::from_block_key(key);

			if (!invert && area.contains_block(pos))
				|| (invert && !area.touches_block(pos))
			{ // If possible, copy whole mapblock.
				let data = idb.get_block(key).unwrap();
				if is_valid_generated(&data) {
					db.set_block(key, &data).unwrap();
				}
			} else { // Copy part of mapblock
				let res = || -> Result<(), MapBlockError> {
					let dst_data = opt_unwrap_or!(
						db.get_block(key).ok()
							.filter(|d| is_valid_generated(&d)),
						return Ok(()));
					let src_data = idb.get_block(key).unwrap();

					let mut src_block = MapBlock::deserialize(&src_data)?;
					let mut dst_block = MapBlock::deserialize(&dst_data)?;
					let mut src_meta = NodeMetadataList::deserialize(
						&src_block.metadata.get_ref())?;
					let mut dst_meta = NodeMetadataList::deserialize(
						&dst_block.metadata.get_ref())?;

					let block_part = area.rel_block_overlap(pos).unwrap();
					if invert {
						// For inverted selections, reverse the order of the
						// overlay operations.
						merge_blocks(&dst_block, &mut src_block,
							block_part, block_part);
						merge_metadata(&dst_meta, &mut src_meta,
							block_part, block_part);
						clean_name_id_map(&mut src_block);
						db.set_block(key, &src_block.serialize()).unwrap();
					} else {
						merge_blocks(&src_block, &mut dst_block,
							block_part, block_part);
						merge_metadata(&src_meta, &mut dst_meta,
							block_part, block_part);
						clean_name_id_map(&mut dst_block);
						db.set_block(key, &dst_block.serialize()).unwrap();
					}
					Ok(())
				}();

				if res.is_err() {
					inst.status.inc_failed()
				}
			}
		} else {
			// No area; copy whole mapblock.
			let data = idb.get_block(key).unwrap();
			if is_valid_generated(&data) {
				db.set_block(key, &data).unwrap();
			}
		}
	}

	inst.status.end_editing();
}


fn get_cached(
	db: &mut MapDatabase,
	cache: &mut CacheMap<i64, Option<MapBlock>>,
	key: i64
) -> Option<MapBlock> {
	match cache.get(&key) {
		Some(data) => data.clone(),
		None => {
			let block = db.get_block(key).ok()
				.filter(|d| is_valid_generated(d))
				.and_then(|d| MapBlock::deserialize(&d).ok());
			cache.insert(key, block.clone());
			block
		}
	}
}


/// Overlay with offset, with or without area.
#[inline]
fn overlay_with_offset(inst: &mut InstBundle) {
	let offset = inst.args.offset.unwrap();
	let src_area = inst.args.area;
	let dst_area = src_area.map(|a| a + offset);
	let idb = inst.idb.as_mut().unwrap();

	// Get keys from output database.
	let dst_keys = query_keys(&mut inst.db, &inst.status,
		&[], dst_area, inst.args.invert, true);

	let mut src_block_cache = CacheMap::with_capacity(BLOCK_CACHE_SIZE);

	inst.status.begin_editing();
	for dst_key in dst_keys {
		inst.status.inc_done();

		let dst_pos = Vec3::from_block_key(dst_key);
		let dst_data = opt_unwrap_or!(
			inst.db.get_block(dst_key).ok().filter(|d| is_valid_generated(d)),
			continue
		);
		let (mut dst_block, mut dst_meta) = unwrap_or!(
			|| -> Result<_, MapBlockError> {
				let b = MapBlock::deserialize(&dst_data)?;
				let m = NodeMetadataList::deserialize(b.metadata.get_ref())?;
				Ok((b, m))
			}(),
			{ inst.status.inc_failed(); continue; }
		);

		let dst_part_abs = dst_area.map_or(
			// If no area is given, the destination part is the whole mapblock.
			Area::new(dst_pos * 16, dst_pos * 16 + 15),
			|a| a.abs_block_overlap(dst_pos).unwrap()
		);
		let src_part_abs = dst_part_abs - offset;
		let src_blocks_needed = src_part_abs.to_touching_block_area();

		for src_pos in &src_blocks_needed {
			if !src_pos.is_valid_block_pos() {
				continue;
			}
			let src_key = src_pos.to_block_key();
			let (src_block, src_meta) = opt_unwrap_or!(
				|| -> Option<_> {
					let b = get_cached(idb, &mut src_block_cache, src_key)?;
					let m = NodeMetadataList::deserialize(b.metadata.get_ref())
						.ok()?;
					Some((b, m))
				}(),
				continue
			);

			let src_frag_abs = src_part_abs.abs_block_overlap(src_pos)
				.unwrap();
			let src_frag_rel = src_frag_abs - src_pos * 16;
			let dst_frag_rel = (src_frag_abs + offset)
				.rel_block_overlap(dst_pos).unwrap();

			merge_blocks(&src_block, &mut dst_block,
				src_frag_rel, dst_frag_rel);
			merge_metadata(&src_meta, &mut dst_meta,
				src_frag_rel, dst_frag_rel);
		}

		clean_name_id_map(&mut dst_block);
		*dst_block.metadata.get_mut() = dst_meta.serialize(dst_block.version);
		inst.db.set_block(dst_key, &dst_block.serialize()).unwrap();
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
			(ArgType::InputMapPath, "Path to the source map/world"),
			(ArgType::Area(false), "Area to copy from. If not specified, \
				everything from the source map will be copied."),
			(ArgType::Invert, "Copy everything *outside* the given area."),
			(ArgType::Offset(false), "Vector to shift nodes by when copying"),
		],
		help: "Copy part or all of a source map into the main map."
	}
}
