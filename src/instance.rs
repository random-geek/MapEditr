use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use anyhow::Context;

use crate::spatial::{Vec3, Area};
use crate::map_database::MapDatabase;
use crate::commands;


#[derive(Clone)]
pub enum ArgType {
	InputMapPath,
	Area(bool),
	Invert,
	Offset(bool),
	Node(bool),
	NewNode,
	Item,
	NewItem,
	Param2Val,
	Object,
	Items,
	Key,
	Value,
}


#[derive(Debug)]
pub struct InstArgs {
	pub command: String,
	pub map_path: String,
	pub input_map_path: Option<String>,
	pub area: Option<Area>,
	pub invert: bool,
	pub offset: Option<Vec3>,
	pub node: Option<String>,
	pub new_node: Option<String>,
	pub item: Option<String>,
	pub new_item: Option<String>,
	pub param2_val: Option<u8>,
	pub object: Option<String>,
	pub items: Option<Vec<String>>,
	pub key: Option<String>,
	pub value: Option<String>,
}


/// Used to tell what sort of progress bar/counter should be shown to the user.
#[derive(Clone, Copy, PartialEq)]
pub enum InstState {
	Ignore,
	Querying,
	Editing
}


#[derive(Clone)]
pub struct InstStatus {
	pub blocks_total: usize,
	pub blocks_done: usize,
	pub state: InstState
}

impl InstStatus {
	fn new() -> Self {
		Self {
			blocks_total: 0,
			blocks_done: 0,
			state: InstState::Ignore
		}
	}
}


pub enum LogType {
	Info,
	Error
}

impl std::fmt::Display for LogType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Info => write!(f, "info"),
			Self::Error => write!(f, "error")
		}
	}
}


pub enum InstEvent {
	NewState(InstState),
	Log(LogType, String)
}


#[derive(Clone)]
pub struct StatusServer {
	status: Arc<Mutex<InstStatus>>,
	event_tx: mpsc::Sender<InstEvent>
}

impl StatusServer {
	pub fn set_state(&self, new_state: InstState) {
		self.status.lock().unwrap().state = new_state;
		self.event_tx.send(InstEvent::NewState(new_state)).unwrap();
	}

	pub fn set_total(&self, total: usize) {
		self.status.lock().unwrap().blocks_total = total;
	}

	pub fn inc_done(&self) {
		self.status.lock().unwrap().blocks_done += 1;
	}

	pub fn begin_editing(&self) {
		self.set_state(InstState::Editing);
	}

	pub fn end_editing(&self) {
		self.set_state(InstState::Ignore);
	}

	pub fn log<S: AsRef<str>>(&self, lt: LogType, msg: S) {
		self.event_tx.send(InstEvent::Log(lt, msg.as_ref().to_string()))
			.unwrap();
	}

	pub fn log_info<S: AsRef<str>>(&self, msg: S) {
		self.log(LogType::Info, msg);
	}

	pub fn log_error<S: AsRef<str>>(&self, msg: S) {
		self.log(LogType::Error, msg);
	}
}


pub struct StatusClient {
	pub event_rx: mpsc::Receiver<InstEvent>,
	status: Arc<Mutex<InstStatus>>
}

impl StatusClient {
	pub fn get(&self) -> InstStatus {
		self.status.lock().unwrap().clone()
	}
}


pub struct InstBundle<'a> {
	pub args: InstArgs,
	pub status: StatusServer,
	pub db: MapDatabase<'a>,
	pub idb: Option<MapDatabase<'a>>
}


fn status_channel() -> (StatusServer, StatusClient) {
	let status1 = Arc::new(Mutex::new(InstStatus::new()));
	let status2 = status1.clone();
	let (event_tx, event_rx) = mpsc::channel();
	(
		StatusServer {status: status1, event_tx},
		StatusClient {status: status2, event_rx}
	)
}


fn verify_args(args: &InstArgs) -> anyhow::Result<()> {
	fn verify_item_name(name: &str) -> anyhow::Result<()> {
		if name == "air" || name == "ignore" {
			Ok(())
		} else {
			let delim = name.find(':')
				.ok_or(anyhow::anyhow!(""))?;

			let mod_name = &name[..delim];
			anyhow::ensure!(mod_name.find(|c: char|
				!(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
			).is_none());

			let item_name = &name[delim + 1..];
			anyhow::ensure!(item_name.find(|c: char|
				!(c.is_ascii_alphanumeric() || c == '_')
			).is_none());

			Ok(())
		}
	}

	if args.area.is_none() && args.invert {
		anyhow::bail!("Cannot invert without a specified area.");
	}
	if let Some(a) = args.area {
		for pos in vec![a.min, a.max] {
			anyhow::ensure!(pos.is_valid_node_pos(),
				"Area corner is outside map bounds: {}.", pos);
		}
	}
	if let Some(sn) = &args.node {
		verify_item_name(sn.as_str())
			.with_context(|| format!("Invalid node name: {}.", sn))?;
	}
	if let Some(rn) = &args.new_node {
		verify_item_name(rn.as_str())
			.with_context(|| format!("Invalid replacement name: {}.", rn))?;
	}

	Ok(())
}


fn open_map(path: PathBuf, flags: sqlite::OpenFlags)
	-> anyhow::Result<sqlite::Connection>
{
	let new_path = if path.is_file() {
		path
	} else {
		let with_file = path.join("map.sqlite");
		if with_file.is_file() {
			with_file
		} else {
			anyhow::bail!("could not find map file");
		}
	};

	Ok(sqlite::Connection::open_with_flags(new_path, flags)?)
}


fn compute_thread(args: InstArgs, status: StatusServer)
	-> anyhow::Result<()>
{
	verify_args(&args)?;

	let commands = commands::get_commands();
	if let Some(cmd_verify) = commands[args.command.as_str()].verify_args {
		cmd_verify(&args)?
	}

	let db_conn = open_map(PathBuf::from(&args.map_path),
		sqlite::OpenFlags::new().set_read_write())?;
	let db = MapDatabase::new(&db_conn)
		.context("Failed to open main world/map.")?;

	let idb_conn = args.input_map_path.as_deref().map(
		|imp| open_map(PathBuf::from(imp),
			sqlite::OpenFlags::new().set_read_only())
	).transpose().context("Failed to open input world/map.")?;
	let idb = match &idb_conn {
		Some(conn) => Some(MapDatabase::new(conn)?),
		None => None
	};

	let func = commands[args.command.as_str()].func;
	let mut inst = InstBundle {args, status, db, idb};
	func(&mut inst);

	if inst.db.is_in_transaction() {
		inst.status.log_info("Committing...");
		inst.db.commit_if_needed()?;
	}
	inst.status.log_info("Done.");
	Ok(())
}


pub fn spawn_compute_thread(args: InstArgs)
	-> (std::thread::JoinHandle<()>, StatusClient)
{
	let (status_tx, status_rx) = status_channel();
	let h = std::thread::Builder::new()
		.name("compute".to_string())
		.spawn(move || {
			compute_thread(args, status_tx.clone()).unwrap_or_else(
				|err| status_tx.log_error(&err.to_string())
			);
		})
		.unwrap();
	(h, status_rx)
}
