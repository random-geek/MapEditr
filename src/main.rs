mod time_keeper;
mod spatial;
mod utils;
mod map_database;
mod map_block;
mod block_utils;
mod instance;
mod commands;
mod cmd_line;


// TODO: Check for unnecessary #derives!
// TODO: Check mapedit TODOs and implement what's needed.
fn main() {
	// TODO: Add GUI. hmm...
	cmd_line::run_cmd_line();
}
