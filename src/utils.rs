use std::time::Duration;

use memmem::{Searcher, TwoWaySearcher};
use byteorder::{WriteBytesExt, BigEndian};

use crate::instance::{InstState, StatusServer};
use crate::map_database::MapDatabase;
use crate::spatial::{Area, Vec3};


pub fn query_keys(
	db: &mut MapDatabase,
	status: &StatusServer,
	// TODO: Allow multiple names for setmetavar and replaceininv.
	search_str: Option<&[u8]>,
	area: Option<Area>,
	invert: bool,
	include_partial: bool
) -> Vec<i64> {
	status.set_state(InstState::Querying);

	// Prepend 16-bit search string length to reduce false positives.
	// This will break if the name-ID map format changes.
	let search_bytes = search_str.map(|s| {
		let mut res = Vec::new();
		res.write_u16::<BigEndian>(s.len() as u16).unwrap();
		res.extend(s);
		res
	});
	let data_searcher = search_bytes.as_ref().map(|b| {
		TwoWaySearcher::new(b)
	});
	let mut keys = Vec::new();

	// Area of included block positions.
	// If invert == true, the function returns only blocks outside this area.
	let block_area = area.map(|a| {
		if invert == include_partial {
			a.to_contained_block_area()
		} else {
			a.to_touching_block_area()
		}
	});

	for (i, (key, data)) in db.iter_rows().enumerate() {
		if let Some(a) = &block_area {
			let block_pos = Vec3::from_block_key(key);
			if a.contains(block_pos) == invert {
				continue;
			}
		}
		if let Some(s) = &data_searcher {
			if s.search_in(&data).is_none() {
				continue;
			}
		}
		keys.push(key);

		// Update total every 1024 iterations.
		if i & 1023 == 0 {
			status.set_total(keys.len())
		}
	}

	status.set_total(keys.len());
	status.set_state(InstState::Ignore);
	keys
}


#[macro_export]
macro_rules! unwrap_or {
	($res:expr, $alt:expr) => {
		match $res {
			Ok(val) => val,
			Err(_) => $alt
		}
	}
}


pub fn fmt_duration(dur: Duration) -> String {
	let s = dur.as_secs();
	if s < 3600 {
		format!("{:02}:{:02}", s / 60 % 60, s % 60)
	} else {
		format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
	}
}


pub fn fmt_big_num(num: u64) -> String {
	let f_num = num as f32;
	let abbrevs = vec![
		("T".to_string(), 1_000_000_000_000.),
		("B".to_string(), 1_000_000_000.),
		("M".to_string(), 1_000_000.),
		("k".to_string(), 1_000.)
	];
	for (suffix, unit) in abbrevs {
		if f_num >= unit {
			let mantissa = f_num / unit;
			let place_vals =
				if mantissa >= 100. { 0 }
				else if mantissa >= 10. { 1 }
				else { 2 };
			return format!("{:.*}{}", place_vals, mantissa, suffix)
		}
	}
	format!("{}", f_num.round())
}
