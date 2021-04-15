use super::{Command, ArgResult, BLOCK_CACHE_SIZE};

use crate::{unwrap_or, opt_unwrap_or};
use crate::spatial::{Vec3, Area, MAP_LIMIT};
use crate::map_database::MapDatabase;
use crate::map_block::{MapBlock, MapBlockError, is_valid_generated,
	NodeMetadataList, NodeMetadataListExt};
use crate::block_utils::{merge_blocks, merge_metadata, clean_name_id_map};
use crate::instance::{ArgType, InstBundle, InstArgs};
use crate::utils::{CacheMap, query_keys};


fn verify_args(args: &InstArgs) -> ArgResult {
	let map_area = Area::new(
		Vec3::new(-MAP_LIMIT, -MAP_LIMIT, -MAP_LIMIT),
		Vec3::new(MAP_LIMIT, MAP_LIMIT, MAP_LIMIT)
	);

	if map_area.intersection(args.area.unwrap() + args.offset.unwrap())
		.is_none()
	{
		return ArgResult::error("Destination area is outside map bounds.");
	}

	ArgResult::Ok
}


type BlockResult = Option<Result<MapBlock, MapBlockError>>;

fn get_cached(
	db: &mut MapDatabase,
	cache: &mut CacheMap<i64, BlockResult>,
	key: i64
) -> BlockResult {
	match cache.get(&key) {
		Some(data) => data.clone(),
		None => {
			let block = db.get_block(key).ok()
				.filter(|d| is_valid_generated(d))
				.map(|d| MapBlock::deserialize(&d));
			cache.insert(key, block.clone());
			block
		}
	}
}


fn clone(inst: &mut InstBundle) {
	let src_area = inst.args.area.unwrap();
	let offset = inst.args.offset.unwrap();
	let dst_area = src_area + offset;
	let mut dst_keys = query_keys(&mut inst.db, &inst.status,
		&[], Some(dst_area), false, true);

	// Sort blocks according to offset such that we don't read blocks that
	// have already been written.
	let sort_dir = offset.map(|v| if v > 0 { -1 } else { 1 });
	// Subtract one from inverted axes to keep values from overflowing.
	let sort_offset = sort_dir.map(|v| if v == -1 { -1 } else { 0 });

	dst_keys.sort_unstable_by_key(|k| {
		(Vec3::from_block_key(*k) * sort_dir + sort_offset).to_block_key()
	});

	let mut block_cache = CacheMap::with_capacity(BLOCK_CACHE_SIZE);
	inst.status.begin_editing();

	for dst_key in dst_keys {
		inst.status.inc_done();

		let (mut dst_block, mut dst_meta) = unwrap_or!(
			opt_unwrap_or!(
				get_cached(&mut inst.db, &mut block_cache, dst_key),
				continue
			).and_then(|b| -> Result<_, MapBlockError> {
				let m = NodeMetadataList::deserialize(b.metadata.get_ref())?;
				Ok((b, m))
			}),
			{ inst.status.inc_failed(); continue; }
		);

		let dst_pos = Vec3::from_block_key(dst_key);
		let dst_part_abs = dst_area.abs_block_overlap(dst_pos).unwrap();
		let src_part_abs = dst_part_abs - offset;
		let src_blocks_needed = src_part_abs.to_touching_block_area();

		for src_pos in &src_blocks_needed {
			if !src_pos.is_valid_block_pos() {
				continue;
			}
			let src_key = src_pos.to_block_key();
			let (src_block, src_meta) = opt_unwrap_or!(
				|| -> Option<_> {
					let b = get_cached(
						&mut inst.db, &mut block_cache, src_key)?.ok()?;
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


pub fn get_command() -> Command {
	Command {
		func: clone,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Area(true), "Area to clone"),
			(ArgType::Offset(true), "Vector to shift the area's contents by")
		],
		help: "Clone (copy) the contents of an area to a new location."
	}
}
