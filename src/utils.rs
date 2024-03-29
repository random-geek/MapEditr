use std::time::Duration;
use std::collections::{HashMap, VecDeque};

use memmem::{Searcher, TwoWaySearcher};
use byteorder::{WriteBytesExt, BigEndian};

use crate::instance::{InstState, StatusServer};
use crate::map_database::MapDatabase;
use crate::spatial::{Area, Vec3};


/// Note: For mapblock version 29 onwards, all block data is compressed, so
/// the `search_strs` argument is ignored.
pub fn query_keys(
	db: &mut MapDatabase,
	status: &StatusServer,
	search_strs: &[Vec<u8>],
	area: Option<Area>,
	invert: bool,
	include_partial: bool
) -> Vec<i64> {
	status.set_state(InstState::Querying);

	// Prepend 16-bit search string length to reduce false positives.
	// This will break if the name-ID map format changes.
	let string16s: Vec<Vec<u8>> = search_strs.iter().map(|s| {
		let mut res = Vec::new();
		res.write_u16::<BigEndian>(s.len() as u16).unwrap();
		res.extend(s);
		res
	}).collect();
	let data_searchers: Vec<TwoWaySearcher> = string16s.iter().map(|b| {
		TwoWaySearcher::new(b)
	}).collect();
	let mut keys = Vec::new();

	// Area of included block positions.
	// If invert == true, the function returns only blocks outside this area.
	let block_area = area.map(|a| {
		if invert == include_partial {
			a.to_contained_block_area()
		} else {
			Some(a.to_touching_block_area())
		}
	}).flatten();
	// True if the given area contains no blocks.
	let empty_area = area.is_some() && block_area.is_none();

	if !empty_area || invert {
		for (i, (key, data)) in db.iter_rows().enumerate() {
			if !empty_area {
				if let Some(a) = &block_area {
					let block_pos = Vec3::from_block_key(key);
					if a.contains(block_pos) == invert {
						continue;
					}
				}
			}
			if let Some(&block_version) = data.get(0) {
				// If block version <= 28, data must match at least one search
				// string. This optimization doesn't work for new mapblocks, as
				// all block data is now compressed.
				// TODO: Remove this legacy optimization?
				if block_version <= 28 && !data_searchers.is_empty()
					&& !data_searchers.iter().any(|s| s.search_in(&data).is_some())
				{
					continue;
				}
			}
			keys.push(key);

			// Update total every 1024 iterations.
			if i & 1023 == 0 {
				status.set_total(keys.len())
			}
		}
	}

	status.set_total(keys.len());
	status.set_state(InstState::Ignore);
	keys
}


pub struct CacheMap<K, V> {
	key_queue: VecDeque<K>,
	map: HashMap<K, V>,
	cap: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V> CacheMap<K, V> {
	pub fn with_capacity(cap: usize) -> Self {
		Self {
			key_queue: VecDeque::with_capacity(cap),
			map: HashMap::with_capacity(cap),
			cap
		}
	}

	pub fn insert(&mut self, key: K, value: V) {
		if self.key_queue.len() >= self.cap {
			if let Some(oldest_key) = self.key_queue.pop_front() {
				self.map.remove(&oldest_key);
			}
		}
		self.key_queue.push_back(key.clone());
		self.map.insert(key, value);
	}

	#[inline]
	pub fn get(&self, key: &K) -> Option<&V> {
		self.map.get(key)
	}
}


pub fn to_bytes(s: &String) -> Vec<u8> {
	s.as_bytes().to_vec()
}


pub fn to_slice(opt: &Option<Vec<u8>>) -> &[Vec<u8>] {
	match opt {
		Some(x) => std::slice::from_ref(x),
		None => &[]
	}
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


#[macro_export]
macro_rules! opt_unwrap_or {
	($res:expr, $alt:expr) => {
		match $res {
			Some(val) => val,
			None => $alt
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
	const ABBREVS: [(&str, f32); 4] = [
		("T", 1_000_000_000_000.),
		("B", 1_000_000_000.),
		("M", 1_000_000.),
		("k", 1_000.)
	];
	for &(suffix, unit) in &ABBREVS {
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


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_nums() {
		let pairs = [
			(0, "0"),
			(3, "3"),
			(42, "42"),
			(999, "999"),
			(1_000, "1.00k"),
			(33_870, "33.9k"),
			(470_999, "471k"),
			(555_678_000, "556M"),
			(1_672_234_000, "1.67B"),
			(77_864_672_234_000, "77.9T"),
		];
		for pair in &pairs {
			assert_eq!(fmt_big_num(pair.0), pair.1.to_string());
		}
	}
}
