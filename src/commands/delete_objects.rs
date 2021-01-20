use super::Command;

use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, LuaEntityData};
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


fn delete_objects(inst: &mut InstBundle) {
	const ITEM_ENT_NAME: &'static [u8] = b"__builtin:item";
	let search_obj = if inst.args.items {
		Some(String::from_utf8(ITEM_ENT_NAME.to_vec()).unwrap())
	} else {
		inst.args.object.clone()
	};
	let keys = query_keys(&mut inst.db, &mut inst.status,
		search_obj.clone(), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();

	let item_searcher = search_obj.as_ref().filter(|_| inst.args.items)
		.map(|s| TwoWaySearcher::new(format!("[itemstring]=\"{}\"", s)));

	let mut count: u64 = 0;
	for key in keys {
		inst.status.inc_done();
		let data = unwrap_or!(inst.db.get_block(key), continue);
		let mut block = unwrap_or!(MapBlock::deserialize(&data), continue);

		let mut modified = false;
		for i in (0..block.static_objects.list.len()).rev() {
			let obj = &block.static_objects.list[i];

			// Check area requirements
			if let Some(area) = inst.args.area {
				const DIV_FAC: i32 = 10_000;
				let rounded_pos = obj.f_pos.map(
					|v| (v - DIV_FAC / 2).div_euclid(DIV_FAC));
				if area.contains(rounded_pos) == inst.args.invert {
					continue;
				}
			}

			// Check name requirements
			let le_data = unwrap_or!(LuaEntityData::deserialize(&obj),
				continue);
			if inst.args.items {
				if le_data.name != ITEM_ENT_NAME {
					continue;
				}
				if let Some(searcher) = &item_searcher {
					if searcher.search_in(&le_data.data).is_none() {
						continue;
					}
				}
			} else {
				if let Some(sobj) = &search_obj {
					if le_data.name != sobj.as_bytes() {
						continue;
					}
				}
			}

			block.static_objects.list.remove(i);
			modified = true;
			count += 1;
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
