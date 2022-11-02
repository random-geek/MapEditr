use super::{Command, ArgResult};

use crate::unwrap_or;
use crate::spatial::Area;
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::{MapBlock, StaticObject, LuaEntityData};
use crate::utils::{query_keys, to_bytes, to_slice, fmt_big_num};

use memmem::{Searcher, TwoWaySearcher};

const ITEM_ENT_NAME: &[u8] = b"__builtin:item";
const ITEM_NAME_PAT_OLD: &[u8] = b"[\"itemstring\"] = \"";
const ITEM_NAME_PAT_NEW: &[u8] = b"itemstring=\"";

thread_local! {
	static ITEM_NAME_SEARCHER_OLD: TwoWaySearcher<'static> =
		TwoWaySearcher::new(ITEM_NAME_PAT_OLD);
	static ITEM_NAME_SEARCHER_NEW: TwoWaySearcher<'static> =
		TwoWaySearcher::new(ITEM_NAME_PAT_NEW);
}


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.object.is_some() && args.items.is_some() {
		return ArgResult::error("Cannot use both --obj and --items.");
	}
	ArgResult::Ok
}


#[inline]
fn get_item_name_start(data: &[u8]) -> Option<usize> {
	if let Some(idx) = ITEM_NAME_SEARCHER_NEW.with(|s| s.search_in(data)) {
		Some(idx + ITEM_NAME_PAT_NEW.len())
	} else if let Some(idx) = ITEM_NAME_SEARCHER_OLD.with(|s| s.search_in(data)) {
		Some(idx + ITEM_NAME_PAT_OLD.len())
	} else {
		None
	}
}


#[inline]
fn get_item_name<'a>(data: &'a [u8]) -> &'a[u8] {
	if data.starts_with(b"return") {
		let item_name_start = get_item_name_start(data);
		if let Some(idx) = item_name_start {
			let name = &data[idx..].split(|&c| c == b' ' || c == b'"').next();
			if let Some(n) = name {
				return n;
			}
		}
		b""
	} else {
		data
	}
}


fn can_delete(
	obj: &StaticObject,
	area: &Option<Area>,
	invert: bool,
	obj_name: &Option<Vec<u8>>,
	item_names: &[Vec<u8>]
) -> bool {
	// Check area requirement
	if let Some(a) = area {
		const DIV_FAC: i32 = 10_000;
		let rounded_pos = obj.f_pos
			.map(|v| (v + DIV_FAC / 2).div_euclid(DIV_FAC));
		if a.contains(rounded_pos) == invert {
			return false; // Object not included in area.
		}
	}

	// Check name requirements
	if let Some(name) = obj_name {
		if let Ok(le_data) = LuaEntityData::deserialize(obj) {
			if &le_data.name != name {
				return false; // Object name does not match.
			}

			if !item_names.is_empty() {
				let item_name = get_item_name(&le_data.data);
				if !item_names.iter().any(|n| n == item_name) {
					// Item entity's item name does not match.
					return false
				}
			}
		} else {
			return false; // Keep invalid or unsupported objects.
		}
	}

	true // Delete if all tests pass.
}


fn delete_objects(inst: &mut InstBundle) {
	let obj_name = if inst.args.items.is_some() {
		Some(ITEM_ENT_NAME.to_owned())
	} else {
		inst.args.object.as_ref().map(to_bytes)
	};

	let item_names: Vec<_> = inst.args.items.as_ref().unwrap_or(&Vec::new())
		.iter().map(to_bytes).collect();

	let keys = query_keys(&mut inst.db, &mut inst.status,
		to_slice(&obj_name), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		let mut modified = false;
		for i in (0 .. block.static_objects.len()).rev() {
			if can_delete(
				&block.static_objects[i],
				&inst.args.area,
				inst.args.invert,
				&obj_name,
				&item_names
			) {
				block.static_objects.remove(i);
				modified = true;
				count += 1;
			}
		}

		if modified {
			inst.db.set_block(key, &block.serialize()).unwrap();
		}
	}

	inst.status.end_editing();
	inst.status.log_info(format!("Deleted {} objects.", fmt_big_num(count)));
}


pub fn get_command() -> Command {
	Command {
		func: delete_objects,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Object, "Name of object to delete"),
			(ArgType::Items,
				"Delete only item entities. Optionally list one or more item \
				names after `--items` to delete only those items."),
			(ArgType::Area(false), "Area in which to delete objects"),
			(ArgType::Invert, "Delete objects *outside* the given area."),
		],
		help: "Delete certain objects and/or item entities."
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_delete_objects() {
		let pairs: &[(&[u8], &[u8])] = &[
			(b"default:glass", b"default:glass"),
			(b"return {}", b""),
			(b"return {[\"itemstring\"] = \"\", [\"age\"] = 100}", b""),
			(b"return {[\"itemstring\"] = \"mod:item\"}", b"mod:item"),
			(b"return {[\"age\"] = 400, [\"itemstring\"] = \"one:two 99 32\"}",
				b"one:two"),
			(b"return {itemstring=\"\",age=100}", b""),
			(b"return {itemstring=\"mod:item\"}", b"mod:item"),
			(b"return {age=400,itemstring=\"one:two 99 32\"}", b"one:two"),
		];
		for &(data, name) in pairs {
			assert_eq!(get_item_name(data), name);
		}
	}
}
