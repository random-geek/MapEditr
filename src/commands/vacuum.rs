use super::Command;

use crate::instance::InstBundle;


fn vacuum(inst: &mut InstBundle) {
	inst.status.log_info("Starting vacuum.");

	inst.status.set_show_progress(false); // No ETA for vacuum.
	inst.status.begin_editing();
	let res = inst.db.vacuum();
	inst.status.end_editing();

	match res {
		Ok(_) => {
			inst.status.log_info(format!("Completed vacuum."));
		},
		Err(e) => inst.status.log_error(format!("Vacuum failed: {}.", e))
	}
}


pub fn get_command() -> Command {
	Command {
		func: vacuum,
		verify_args: None,
		args: Vec::new(),
		help: "Rebuild map database to reduce its size."
	}
}
