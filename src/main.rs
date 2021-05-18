// Uncomment if needed for testing
// mod testing;
mod spatial;
mod utils;
mod map_database;
mod map_block;
mod block_utils;
mod instance;
mod commands;
mod cmd_line;


fn main() {
	// TODO: Add a GUI. hmm...
	cmd_line::run_cmd_line();
}
