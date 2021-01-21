use super::Command;

use crate::spatial::Area;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, StaticObject, LuaEntityData};
use crate::utils::{query_keys, fmt_big_num};

use memmem::{Searcher, TwoWaySearcher};


macro_rules! unwrap_or {
	($res:expr, $alt:expr) => {
		match $res {
			Ok(val) => val,
			Err(_) => $alt
		}
	}
}


#[inline]
fn can_delete(
	obj: &StaticObject,
	area: &Option<Area>,
	invert: bool
) -> bool {
	// Check area requirements
	if let Some(a) = area {
		const DIV_FAC: i32 = 10_000;
		let rounded_pos = obj.f_pos.map(
			|v| (v - DIV_FAC / 2).div_euclid(DIV_FAC));
		if a.contains(rounded_pos) == invert {
			return false;
		}
	}

	true
}


fn delete_objects(inst: &mut InstBundle) {
	const ITEM_ENT_NAME: &[u8] = b"__builtin:item";
	let search_obj = if inst.args.items {
		Some(ITEM_ENT_NAME.to_owned())
	} else {
		inst.args.object.as_ref().map(|s| s.as_bytes().to_owned())
	};
	let keys = query_keys(&mut inst.db, &mut inst.status,
		search_obj.as_deref(), inst.args.area, inst.args.invert, true);

	let search_item = search_obj.as_ref().filter(|_| inst.args.items).map(|s|
		format!(
			"[\"itemstring\"] = \"{}\"",
			String::from_utf8(s.to_owned()).unwrap()
		).into_bytes()
	);
	let item_searcher = search_item.as_ref().map(|s| TwoWaySearcher::new(s));

	inst.status.begin_editing();
	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();
		let data = unwrap_or!(inst.db.get_block(key), continue);
		let mut block = unwrap_or!(MapBlock::deserialize(&data), continue);

		let mut modified = false;
		for i in (0..block.static_objects.list.len()).rev() {
			let obj = &block.static_objects.list[i];

			if can_delete(
				&block.static_objects.list[i],
				&inst.args.area,
				inst.args.invert
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
				"Name of object (or item with --item) to search for."),
			(ArgType::Items,
				"Delete dropped items using object name as item name."),
		],
		help: "Delete certain objects (entities)."
	}
}
