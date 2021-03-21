use std::io::prelude::*;
use std::time::{Duration, Instant};

use clap::{App, Arg, SubCommand, AppSettings, crate_version, crate_authors};
use anyhow::Context;

use crate::spatial::{Vec3, Area};
use crate::instance::{LogType, ArgType, InstArgs};
use crate::commands::{get_commands};
use crate::utils::fmt_duration;


fn arg_to_pos(p: clap::Values) -> anyhow::Result<Vec3> {
	let vals: Vec<_> = p.collect();
	if vals.len() != 3 {
		anyhow::bail!("");
	}
	Ok(Vec3::new(
		vals[0].parse()?,
		vals[1].parse()?,
		vals[2].parse()?
	))
}


fn to_cmd_line_args<'a>(tup: &(ArgType, &'a str))
	-> Vec<Arg<'a, 'a>>
{
	let arg_type = tup.0.clone();
	let help_msg = tup.1;
	if let ArgType::Area(req) = arg_type {
		return vec![
			Arg::with_name("p1")
				.long("p1")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req)
				.requires("p2")
				.help(help_msg),
			Arg::with_name("p2")
				.long("p2")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req)
				.requires("p1")
				.help(help_msg)
		];
	}
	// TODO: Ensure arguments are correctly defined.
	let arg = match arg_type {
		ArgType::Area(_) => unreachable!(),
		ArgType::InputMapPath =>
			Arg::with_name("input_map")
				.required(true),
		ArgType::Invert =>
			Arg::with_name("invert")
				.long("invert"),
		ArgType::Offset(req) =>
			Arg::with_name("offset")
				.long("offset")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req),
		ArgType::Node(req) => {
			let a = Arg::with_name("node")
				.required(req);
			if req {
				a
			} else {
				a.long("node").takes_value(true)
			}
		},
		ArgType::Nodes =>
			Arg::with_name("nodes")
				.long("nodes")
				.min_values(1),
		ArgType::NewNode =>
			Arg::with_name("new_node")
				.takes_value(true)
				.required(true),
		ArgType::Object =>
			Arg::with_name("object")
				.long("obj")
				.takes_value(true),
		ArgType::Item =>
			Arg::with_name("item")
				.takes_value(true)
				.required(true),
		ArgType::Items =>
			Arg::with_name("items")
				.long("items")
				.min_values(0)
				.max_values(1),
		ArgType::NewItem =>
			Arg::with_name("new_item")
				.takes_value(true),
		ArgType::DeleteMeta =>
			Arg::with_name("delete_meta")
				.long("deletemeta"),
		ArgType::DeleteItem =>
			Arg::with_name("delete_item")
				.long("delete"),
		ArgType::Key =>
			Arg::with_name("key")
				.takes_value(true)
				.required(true),
		ArgType::Value =>
			Arg::with_name("value")
				.takes_value(true)
				.required(true),
		ArgType::Param2Val =>
			Arg::with_name("param2_val")
				.required(true),
	}.help(help_msg);

	vec![arg]
}


fn parse_cmd_line_args() -> anyhow::Result<InstArgs> {
	/* Create the clap app */
	let commands = get_commands();

	let app_commands = commands.iter().map(|(cmd_name, cmd)| {
		let args: Vec<_> = cmd.args.iter().flat_map(to_cmd_line_args)
			.collect();
		SubCommand::with_name(cmd_name)
			.about(cmd.help)
			.args(&args)
	});

	let app = App::new("MapEditr")
		.about("Edits Minetest worlds/map databases.")
		.after_help(
			"For command-specific help, run: mapeditr <SUBCOMMAND> -h\n\
			For additional information, see the manual.")
		.version(crate_version!())
		.author(crate_authors!())
		.arg(Arg::with_name("yes")
			.long("yes")
			.short("y")
			.global(true)
			.help("Skip the default confirmation prompt.")
		)
		// TODO: Move map arg to subcommands?
		.arg(Arg::with_name("map")
			.required(true)
			.help("Path to world directory or map database to edit.")
		)
		.setting(AppSettings::SubcommandRequired)
		.subcommands(app_commands);

	/* Parse the arguments */
	let matches = app.get_matches();
	let sub_name = matches.subcommand_name().unwrap().to_string();
	let sub_matches = matches.subcommand_matches(&sub_name).unwrap();

	Ok(InstArgs {
		do_confirmation: !matches.is_present("yes"),
		command: sub_name,
		map_path: matches.value_of("map").unwrap().to_string(),
		input_map_path: sub_matches.value_of("input_map").map(str::to_string),
		area: {
			let p1_maybe = sub_matches.values_of("p1").map(arg_to_pos)
				.transpose().context("Invalid p1 value")?;
			let p2_maybe = sub_matches.values_of("p2").map(arg_to_pos)
				.transpose().context("Invalid p2 value")?;
			if let (Some(p1), Some(p2)) = (p1_maybe, p2_maybe) {
				Some(Area::from_unsorted(p1, p2))
			} else {
				None
			}
		},
		invert: sub_matches.is_present("invert"),
		offset: sub_matches.values_of("offset").map(arg_to_pos).transpose()
			.context("Invalid offset value")?,
		node: sub_matches.value_of("node").map(str::to_string),
		nodes: sub_matches.values_of("nodes").iter_mut().flatten()
			.map(str::to_string).collect(),
		new_node: sub_matches.value_of("new_node").map(str::to_string),
		object: sub_matches.value_of("object").map(str::to_string),
		item: sub_matches.value_of("item").map(str::to_string),
		items: sub_matches.values_of("items")
			.map(|v| v.map(str::to_string).collect()),
		new_item: sub_matches.value_of("new_item").map(str::to_string),
		delete_meta: sub_matches.is_present("delete_meta"),
		delete_item: sub_matches.is_present("delete_item"),
		key: sub_matches.value_of("key").map(str::to_string),
		value: sub_matches.value_of("value").map(str::to_string),
		param2_val: sub_matches.value_of("param2_val").map(|val| val.parse())
			.transpose().context("Invalid param2 value.")?,
	})
}


fn print_editing_status(done: usize, total: usize, real_start: Instant,
	eta_start: Instant, show_progress: bool)
{
	let now = Instant::now();
	let real_elapsed = now.duration_since(real_start);

	if show_progress {
		let eta_elapsed = now.duration_since(eta_start);
		let progress = match total {
			0 => 0.,
			_ => done as f32 / total as f32
		};

		let remaining = if progress >= 0.1 {
			Some(Duration::from_secs_f32(
				eta_elapsed.as_secs_f32() / progress * (1. - progress)
			))
		} else {
			None
		};

		const TOTAL_BARS: usize = 25;
		let num_bars = (progress * TOTAL_BARS as f32) as usize;
		let bars = "=".repeat(num_bars);

		print!(
			"\r[{bars:<total_bars$}] {progress:.1}% | {elapsed} elapsed \
				| {remaining} remaining",
			bars=bars,
			total_bars=TOTAL_BARS,
			progress=progress * 100.,
			elapsed=fmt_duration(real_elapsed),
			remaining=if let Some(d) = remaining {
				fmt_duration(d)
			} else {
				String::from("--:--")
			}
		);
	} else {
		print!("\rProcessing... {} elapsed", fmt_duration(real_elapsed));
	}

	std::io::stdout().flush().unwrap();
}


fn print_log(log_type: LogType, msg: String) {
	let prefix = format!("{}: ", log_type);
	let indented = msg.lines().collect::<Vec<_>>()
		.join(&format!( "\n{}", " ".repeat(prefix.len()) ));
	println!("{}{}", prefix, indented);
}


fn get_confirmation() -> bool {
	print!("Proceed? (Y/n): ");
	let mut result = String::new();
	std::io::stdin().read_line(&mut result).unwrap();
	result.trim().to_ascii_lowercase() == "y"
}


pub fn run_cmd_line() {
	use std::sync::mpsc;
	use crate::instance::{InstState, ServerEvent, spawn_compute_thread};

	let args = match parse_cmd_line_args() {
		Ok(a) => a,
		Err(e) => {
			print_log(LogType::Error, e.to_string());
			return;
		}
	};
	let (handle, status) = spawn_compute_thread(args);

	const TICK: Duration = Duration::from_millis(25);
	const UPDATE_INTERVAL: Duration = Duration::from_millis(500);

	let mut last_update = Instant::now();
	let mut querying_start = last_update;
	let mut editing_start = last_update;
	let mut cur_state = InstState::Ignore;
	let mut need_newline = false;

	let newline_if = |condition: &mut bool| {
		if *condition {
			println!();
			*condition = false;
		}
	};

	loop { /* Main command-line logging loop */
		let now = Instant::now();
		let mut forced_update = InstState::Ignore;

		match status.receiver().recv_timeout(TICK) {
			Ok(event) => match event {
				ServerEvent::Log(log_type, msg) => {
					newline_if(&mut need_newline);
					print_log(log_type, msg);
				},
				ServerEvent::NewState(new_state) => {
					// Force progress updates at the beginning and end of
					// querying/editing stages.
					if (cur_state == InstState::Ignore) !=
						(new_state == InstState::Ignore)
					{
						forced_update =
							if cur_state == InstState::Ignore { new_state }
							else { cur_state };
					}
					if new_state == InstState::Querying {
						// Store time for determining elapsed time.
						querying_start = now;
					} else if new_state == InstState::Editing {
						// Store start time for determining ETA.
						editing_start = now;
					}
					cur_state = new_state;
				},
				ServerEvent::ConfirmRequest => {
					newline_if(&mut need_newline);
					status.confirm(get_confirmation());
				},
			},
			Err(err) => {
				// Compute thread has exited; break out of the loop.
				if err == mpsc::RecvTimeoutError::Disconnected {
					break;
				}
			}
		}

		let timed_update_ready = now >= last_update + UPDATE_INTERVAL;

		if forced_update == InstState::Querying
			|| (cur_state == InstState::Querying && timed_update_ready)
		{
			print!("\rQuerying mapblocks... {} found.",
				status.get_status().blocks_total);
			std::io::stdout().flush().unwrap();
			last_update = now;
			need_newline = true;
		}
		else if forced_update == InstState::Editing
			|| (cur_state == InstState::Editing && timed_update_ready)
		{
			let s = status.get_status();
			print_editing_status(s.blocks_done, s.blocks_total,
				querying_start, editing_start, s.show_progress);
			last_update = now;
			need_newline = true;
		}

		// Print a newline after the last querying/editing message.
		if cur_state == InstState::Ignore {
			newline_if(&mut need_newline);
		}
	}

	let _ = handle.join();
}
