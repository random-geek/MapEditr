use std::io::prelude::*;
use std::time::{Duration, Instant};

use clap::{App, Arg, SubCommand, AppSettings, crate_version, crate_authors};
use anyhow::Context;

use crate::spatial::{Vec3, Area};
use crate::instance::{ArgType, InstArgs};
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
	let arg = tup.0.clone();
	let help = tup.1;
	if let ArgType::Area(req) = arg {
		return vec![
			Arg::with_name("p1")
				.long("p1")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req)
				.requires("p2")
				.help(help),
			Arg::with_name("p2")
				.long("p2")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req)
				.requires("p1")
				.help(help)
		];
	}
	vec![match arg {
		// TODO: Remove unused conditions here.
		ArgType::InputMapPath =>
			Arg::with_name("input_map")
				.required(true)
				.help(help),
		ArgType::Area(_) => unreachable!(),
		ArgType::Invert =>
			Arg::with_name("invert")
				.long("invert")
				.help(help),
		ArgType::Offset(req) =>
			Arg::with_name("offset")
				.long("offset")
				.allow_hyphen_values(true)
				.number_of_values(3)
				.value_names(&["x", "y", "z"])
				.required(req)
				.help(help),
		ArgType::Node(req) => {
			let a = Arg::with_name("node")
				.required(req)
				.help(help);
			if !req {
				a.long("node").takes_value(true)
			} else {
				a
			}
		},
		ArgType::NewNode(req) => {
			let a = Arg::with_name("new_node")
				.required(req)
				.help(help);
			if !req {
				a.long("newnode").takes_value(true)
			} else {
				a
			}
		},
		ArgType::Item =>
			Arg::with_name("item")
				.takes_value(true)
				.required(true)
				.help(help),
		ArgType::NewItem =>
			Arg::with_name("new_item")
				.takes_value(true)
				.required(true)
				.help(help),
		ArgType::Param2Val(_) =>
			Arg::with_name("param2_val")
				.required(true)
				.help(help),
		ArgType::Object(req) =>
			Arg::with_name("object")
				.long("obj")
				.takes_value(true)
				.required(req)
				.help(help),
		ArgType::Items =>
			Arg::with_name("items")
				.long("items")
				.min_values(0)
				.max_values(1)
				.help(help),
		ArgType::Key =>
			Arg::with_name("key")
				.takes_value(true)
				.required(true)
				.help(help),
		ArgType::Value =>
			Arg::with_name("value")
				.takes_value(true)
				.required(true)
				.help(help),
	}]
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
		.after_help("For command-specific help, run: mapeditr <command> -h")
		.version(crate_version!())
		.author(crate_authors!())
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
		map_path: matches.value_of("map").unwrap().to_string(),
		command: sub_name,
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
		new_node: sub_matches.value_of("new_node").map(str::to_string),
		item: sub_matches.value_of("item").map(str::to_string),
		new_item: sub_matches.value_of("new_item").map(str::to_string),
		param2_val: sub_matches.value_of("param2_val")
			.map(|v| v.parse().unwrap()),
		object: sub_matches.value_of("object").map(str::to_string),
		items: sub_matches.values_of("items")
			.map(|v| v.map(str::to_string).collect()),
		key: sub_matches.value_of("key").map(str::to_string),
		value: sub_matches.value_of("value").map(str::to_string),
	})
}


fn print_progress(done: usize, total: usize, real_start: Instant,
	eta_start: Instant)
{
	let progress = match total {
		0 => 0.0,
		_ => done as f32 / total as f32
	};

	let now = Instant::now();
	let real_elapsed = now.duration_since(real_start);
	let eta_elapsed = now.duration_since(eta_start);

	let remaining = if progress >= 0.1 {
		Some(Duration::from_secs_f32(
			eta_elapsed.as_secs_f32() / progress * (1.0 - progress)
		))
	} else {
		None
	};

	const TOTAL_BARS: usize = 25;
	let num_bars = (progress * TOTAL_BARS as f32) as usize;
	let bars = "=".repeat(num_bars);

	eprint!(
		"\r[{bars:<total_bars$}] {progress:.1}% | {elapsed} elapsed \
			| {remaining} remaining",
		bars=bars,
		total_bars=TOTAL_BARS,
		progress=progress * 100.0,
		elapsed=fmt_duration(real_elapsed),
		remaining=if let Some(d) = remaining {
			fmt_duration(d)
		} else {
			String::from("--:--")
		}
	);

	std::io::stdout().flush().unwrap();
}


pub fn run_cmd_line() {
	use std::sync::mpsc;
	use crate::instance::{InstState, InstEvent, spawn_compute_thread};

	let args = parse_cmd_line_args().unwrap();
	let (handle, status) = spawn_compute_thread(args);

	const TICK: Duration = Duration::from_millis(25);
	const UPDATE_INTERVAL: Duration = Duration::from_millis(250);

	let mut querying_start = Instant::now();
	let mut editing_start = Instant::now();
	let mut last_update = Instant::now();
	let mut cur_state = InstState::Ignore;
	let mut last_printed = InstState::Ignore;

	loop { /* Main command-line logging loop */
		let now = Instant::now();
		let mut forced_update = InstState::Ignore;

		match status.event_rx.recv_timeout(TICK) {
			Ok(event) => match event {
				InstEvent::NewState(new_state) => {
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
				InstEvent::Log(log_type, msg) => {
					if last_printed != InstState::Ignore {
						eprintln!();
					}
					last_printed = InstState::Ignore;
					eprintln!("{}: {}", log_type, msg);
				}
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
			eprint!("\rQuerying map blocks... {} found.",
				status.get().blocks_total);
			last_update = now;
			last_printed = InstState::Querying;
		}
		else if forced_update == InstState::Editing
			|| (cur_state == InstState::Editing && timed_update_ready)
		{
			if last_printed == InstState::Querying {
				eprintln!();
			}
			last_printed = InstState::Editing;
			let s = status.get();
			print_progress(s.blocks_done, s.blocks_total,
				querying_start, editing_start);
			last_update = now;
		}
	}

	if last_printed != InstState::Ignore {
		eprintln!("");
	}

	let _ = handle.join();
}
