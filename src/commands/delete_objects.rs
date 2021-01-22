use super::Command;

use crate::unwrap_or;
use crate::spatial::Area;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, StaticObject, LuaEntityData};
use crate::utils::{query_keys, fmt_big_num};

use memmem::{Searcher, TwoWaySearcher};

const ITEM_ENT_NAME: &[u8] = b"__builtin:item";


#[inline]
fn can_delete(
	obj: &StaticObject,
	area: &Option<Area>,
	invert: bool,
	obj_name: &Option<Vec<u8>>,
	item_searcher: &Option<TwoWaySearcher>
) -> bool {
	// Check area requirement
	if let Some(a) = area {
		const DIV_FAC: i32 = 10_000;
		let rounded_pos = obj.f_pos.map(
			|v| (v - DIV_FAC / 2).div_euclid(DIV_FAC));
		if a.contains(rounded_pos) == invert {
			return false; // Object not included in area.
		}
	}

	// Check name requirement
	if let Some(n) = obj_name {
		if let Ok(le_data) = LuaEntityData::deserialize(obj) {
			if &le_data.name != n {
				return false; // Object name does not match.
			}

			if let Some(is) = item_searcher {
				if is.search_in(&le_data.data).is_none() {
					return false; // Item entity name does not match.
				}
			}
		} else {
			return false; // Unsupported object type, don't delete it.
		}
	}

	true // Delete if all tests pass.
}


fn delete_objects(inst: &mut InstBundle) {
	let search_obj = if inst.args.items.is_some() {
		Some(ITEM_ENT_NAME.to_owned())
	} else {
		inst.args.object.as_ref().map(|s| s.as_bytes().to_owned())
	};

	// search_item will be Some if (1) item search is enabled and (2) an item
	// is specified.
	let search_item = inst.args.items.as_ref()
		.and_then(|items| items.get(0))
		.map(|s| s.as_bytes().to_owned());
	let item_searcher = search_item.as_ref()
		.map(|s| TwoWaySearcher::new(s));

	let keys = query_keys(&mut inst.db, &mut inst.status,
		search_obj.as_deref(), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();
		let data = unwrap_or!(inst.db.get_block(key), continue);
		let mut block = unwrap_or!(MapBlock::deserialize(&data), continue);

		let mut modified = false;
		for i in (0..block.static_objects.list.len()).rev() {
			if can_delete(
				&block.static_objects.list[i],
				&inst.args.area,
				inst.args.invert,
				&search_obj,
				&item_searcher
			) {
				block.static_objects.list.remove(i);
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
		verify_args: None,
		args: vec![
			(ArgType::Area(false), "Area in which to delete objects"),
			(ArgType::Invert, "Delete all objects outside the area"),
			(ArgType::Object(false),
				"Name of object to delete. If not specified, all objects will \
				be deleted"),
			(ArgType::Items,
				"Delete item entities. Optionally specify an item name to \
				delete."),
		],
		help: "Delete certain objects (entities)."
	}
}
