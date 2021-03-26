use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use anyhow::Context;

use crate::spatial::{Vec3, Area, MAP_LIMIT};
use crate::map_database::MapDatabase;
use crate::commands;
use crate::commands::ArgResult;


#[derive(Clone)]
pub enum ArgType {
	InputMapPath,
	Area(bool),
	Invert,
	Offset(bool),
	Node(bool),
	Nodes,
	NewNode,
	Object,
	Item,
	Items,
	NewItem,
	Delete,
	DeleteMeta,
	Key,
	Value,
	Param2,
}


#[derive(Debug)]
pub struct InstArgs {
	pub do_confirmation: bool,
	pub command: String,
	pub map_path: String,
	pub input_map_path: Option<String>,
	pub area: Option<Area>,
	pub invert: bool,
	pub offset: Option<Vec3>,
	pub node: Option<String>,
	pub nodes: Vec<String>,
	pub new_node: Option<String>,
	pub object: Option<String>,
	pub item: Option<String>,
	pub items: Option<Vec<String>>,
	pub new_item: Option<String>,
	pub delete: bool,
	pub delete_meta: bool,
	pub key: Option<String>,
	pub value: Option<String>,
	pub param2: Option<u8>,
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
	pub show_progress: bool,
	pub blocks_total: usize,
	pub blocks_done: usize,
	pub blocks_failed: usize,
	pub state: InstState
}

impl InstStatus {
	fn new() -> Self {
		Self {
			show_progress: true,
			blocks_total: 0,
			blocks_done: 0,
			blocks_failed: 0,
			state: InstState::Ignore
		}
	}
}


pub enum LogType {
	Info,
	Warning,
	Error
}

impl std::fmt::Display for LogType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Info => write!(f, "info"),
			Self::Warning => write!(f, "warning"),
			Self::Error => write!(f, "error")
		}
	}
}


pub enum ServerEvent {
	Log(LogType, String),
	NewState(InstState),
	ConfirmRequest,
}


pub enum ClientEvent {
	ConfirmResponse(bool),
}


pub struct StatusServer {
	status: Arc<Mutex<InstStatus>>,
	event_tx: mpsc::Sender<ServerEvent>,
	event_rx: mpsc::Receiver<ClientEvent>,
}

impl StatusServer {
	pub fn get_status(&self) -> InstStatus {
		self.status.lock().unwrap().clone()
	}

	pub fn set_state(&self, new_state: InstState) {
		self.status.lock().unwrap().state = new_state;
		self.event_tx.send(ServerEvent::NewState(new_state)).unwrap();
	}

	pub fn set_total(&self, total: usize) {
		self.status.lock().unwrap().blocks_total = total;
	}

	pub fn inc_done(&self) {
		self.status.lock().unwrap().blocks_done += 1;
	}

	pub fn inc_failed(&mut self) {
		self.status.lock().unwrap().blocks_failed += 1;
	}

	pub fn set_show_progress(&self, sp: bool) {
		self.status.lock().unwrap().show_progress = sp;
	}

	pub fn begin_editing(&self) {
		self.set_state(InstState::Editing);
	}

	pub fn end_editing(&self) {
		self.set_state(InstState::Ignore);
	}

	pub fn get_confirmation(&self) -> bool {
		self.event_tx.send(ServerEvent::ConfirmRequest).unwrap();
		while let Ok(event) = self.event_rx.recv() {
			match event {
				ClientEvent::ConfirmResponse(res) => return res
			}
		}
		false
	}

	fn log<S: AsRef<str>>(&self, lt: LogType, msg: S) {
		self.event_tx.send(ServerEvent::Log(lt, msg.as_ref().to_string()))
			.unwrap();
	}

	pub fn log_info<S: AsRef<str>>(&self, msg: S) {
		self.log(LogType::Info, msg);
	}

	pub fn log_warning<S: AsRef<str>>(&self, msg: S) {
		self.log(LogType::Warning, msg);
	}

	pub fn log_error<S: AsRef<str>>(&self, msg: S) {
		self.log(LogType::Error, msg);
	}
}


pub struct StatusClient {
	status: Arc<Mutex<InstStatus>>,
	event_tx: mpsc::Sender<ClientEvent>,
	event_rx: mpsc::Receiver<ServerEvent>,
}

impl StatusClient {
	pub fn get_status(&self) -> InstStatus {
		self.status.lock().unwrap().clone()
	}

	#[inline]
	pub fn receiver(&self) -> &mpsc::Receiver<ServerEvent> {
		&self.event_rx
	}

	pub fn confirm(&self, choice: bool) {
		self.event_tx.send(ClientEvent::ConfirmResponse(choice)).unwrap();
	}
}


fn status_link() -> (StatusServer, StatusClient) {
	let status1 = Arc::new(Mutex::new(InstStatus::new()));
	let status2 = status1.clone();
	let (s_event_tx, s_event_rx) = mpsc::channel();
	let (c_event_tx, c_event_rx) = mpsc::channel();
	(
		StatusServer {
			status: status1,
			event_tx: s_event_tx,
			event_rx: c_event_rx,
		},
		StatusClient {
			status: status2,
			event_tx: c_event_tx,
			event_rx: s_event_rx,
		}
	)
}


pub struct InstBundle<'a> {
	pub args: InstArgs,
	pub status: StatusServer,
	pub db: MapDatabase<'a>,
	pub idb: Option<MapDatabase<'a>>
}


fn verify_args(args: &InstArgs) -> anyhow::Result<()> {
	// TODO: Complete verifications.

	if args.area.is_none() && args.invert {
		anyhow::bail!("Cannot invert without a specified area.");
	}
	if let Some(a) = args.area {
		for pos in &[a.min, a.max] {
			anyhow::ensure!(pos.is_valid_node_pos(),
				"Area corner is outside map bounds: {}.", pos);
		}
	}
	if let Some(offset) = args.offset {
		let huge = |n| n < -MAP_LIMIT * 2 || n > MAP_LIMIT * 2;

		if huge(offset.x) || huge(offset.y) || huge(offset.z) {
			anyhow::bail!(
				"Offset cannot be larger than {} nodes in any direction.",
				MAP_LIMIT * 2);
		}
	}

	fn is_valid_name(name: &str) -> bool {
		if name == "air" || name == "ignore" {
			true
		} else {
			let delim = match name.find(':') {
				Some(d) => d,
				None => return false
			};

			let mod_name = &name[..delim];
			let item_name = &name[delim + 1..];

			mod_name.find(|c: char|
				!(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
				.is_none()
			&& item_name.find(|c: char|
				!(c.is_ascii_alphanumeric() || c == '_'))
				.is_none()
		}
	}

	macro_rules! verify_name {
		($name:expr, $msg:literal) => {
			if let Some(n) = &$name {
				anyhow::ensure!(is_valid_name(n), $msg, n);
			}
		}
	}

	verify_name!(args.node, "Invalid node name: {}");
	for n in &args.nodes {
		anyhow::ensure!(is_valid_name(n), "Invalid node name: {}", n);
	}
	verify_name!(args.new_node, "Invalid node name: {}");
	verify_name!(args.object, "Invalid object name: {}");
	verify_name!(args.item, "Invalid item name: {}");
	if let Some(items) = &args.items {
		for i in items {
			anyhow::ensure!(is_valid_name(i), "Invalid item name: {}", i);
		}
	}
	verify_name!(args.new_item, "Invalid item name: {}");
	// TODO: Are keys/values escaped?

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


fn compute_thread(args: InstArgs, status: StatusServer) -> anyhow::Result<()> {
	verify_args(&args)?;

	let commands = commands::get_commands();
	let mut cmd_warning = None;
	if let Some(cmd_verify) = commands[args.command.as_str()].verify_args {
		cmd_warning = match cmd_verify(&args) {
			ArgResult::Ok => None,
			ArgResult::Warning(w) => Some(w),
			ArgResult::Error(e) => anyhow::bail!(e)
		}
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

	// Issue warnings and confirmation prompt.
	if inst.args.do_confirmation {
		inst.status.log_warning(
			"This tool can permanently damage your Minetest world.\n\
			Always EXIT Minetest and BACK UP the map database before use.");
	}
	if let Some(w) = cmd_warning {
		inst.status.log_warning(w);
	}
	if inst.args.do_confirmation && !inst.status.get_confirmation() {
		return Ok(());
	}

	func(&mut inst); // The real thing!

	let fails = inst.status.get_status().blocks_failed;
	if fails > 0 {
		inst.status.log_info(format!(
			"Skipped {} invalid/unsupported mapblocks.", fails));
	}

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
	let (status_server, status_client) = status_link();
	// Clone within this thread to avoid issue #39364 (hopefully).
	let raw_event_tx = status_server.event_tx.clone();
	let h = std::thread::Builder::new()
		.name("compute".to_string())
		.spawn(move || {
			compute_thread(args, status_server).unwrap_or_else(
				// TODO: Find a cleaner way to do this.
				|err| raw_event_tx.send(
					ServerEvent::Log(LogType::Error, err.to_string())).unwrap()
			);
		})
		.unwrap();
	(h, status_client)
}
