use super::Command;

use std::time::Instant;
use crate::instance::InstBundle;
use crate::utils::fmt_duration;


fn vacuum(inst: &mut InstBundle) {
	inst.status.log_info("Starting vacuum.");
	let start = Instant::now();

	// TODO: Show simple timer in main thread.
	match inst.db.vacuum() {
		Ok(_) => {
			let time = fmt_duration(start.elapsed());
			inst.status.log_info(format!("Completed vacuum in {}.", time));
		},
		Err(e) => inst.status.log_error(format!("Vacuum failed: {}.", e))
	}
}


pub fn get_command() -> Command {
	Command {
		func: vacuum,
		verify_args: None,
		args: vec![],
		help: "Rebuild map database to reduce its size"
	}
}
