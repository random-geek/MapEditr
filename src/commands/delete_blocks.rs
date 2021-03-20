use super::Command;

use crate::instance::{ArgType, InstBundle};
use crate::utils::query_keys;


fn delete_blocks(inst: &mut InstBundle) {
	let keys = query_keys(&mut inst.db, &inst.status,
		&[], inst.args.area, inst.args.invert, false);
	inst.status.begin_editing();

	for key in keys {
		inst.status.inc_done();
		inst.db.delete_block(key).unwrap();
	}

	inst.status.end_editing();
}


pub fn get_command() -> Command {
	Command {
		func: delete_blocks,
		verify_args: None,
		args: vec![
			(ArgType::Area(true), "Area containing mapblocks to delete"),
			(ArgType::Invert,
				"Delete all mapblocks fully *outside* the given area.")
		],
		help: "Delete all mapblocks inside or outside an area."
	}
}
