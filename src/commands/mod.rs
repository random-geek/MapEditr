use std::collections::BTreeMap;

use crate::instance::{ArgType, InstArgs, InstBundle};

mod clone;
mod delete_blocks;
mod delete_meta;
mod delete_objects;
mod delete_timers;
mod fill;
mod overlay;
mod replace_in_inv;
mod replace_nodes;
mod set_meta_var;
mod set_param2;
mod vacuum;


pub const BLOCK_CACHE_SIZE: usize = 1024;


pub enum ArgResult {
	Ok,
	Warning(String),
	Error(String),
}

impl ArgResult {
	/// Create a new ArgResult::Warning from a &str.
	#[inline]
	pub fn warning(msg: &str) -> Self {
		Self::Warning(msg.to_string())
	}

	/// Create a new ArgResult::Error from a &str.
	#[inline]
	pub fn error(msg: &str) -> Self {
		Self::Error(msg.to_string())
	}
}


pub struct Command {
	pub func: fn(&mut InstBundle),
	pub verify_args: Option<fn(&InstArgs) -> ArgResult>,
	pub help: &'static str,
	pub args: Vec<(ArgType, &'static str)>
}


pub fn get_commands() -> BTreeMap<&'static str, Command> {
	let mut commands = BTreeMap::new();
	macro_rules! new_cmd {
		($name:expr, $module:ident) => {
			commands.insert($name, $module::get_command())
		}
	}

	new_cmd!("clone", clone);
	new_cmd!("deleteblocks", delete_blocks);
	new_cmd!("deletemeta", delete_meta);
	new_cmd!("deleteobjects", delete_objects);
	new_cmd!("deletetimers", delete_timers);
	new_cmd!("fill", fill);
	new_cmd!("replacenodes", replace_nodes);
	new_cmd!("replaceininv", replace_in_inv);
	new_cmd!("overlay", overlay);
	new_cmd!("setmetavar", set_meta_var);
	new_cmd!("setparam2", set_param2);
	new_cmd!("vacuum", vacuum);

	commands
}
