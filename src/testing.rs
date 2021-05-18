use std::collections::HashMap;
use std::time::{Instant, Duration};

use crate::instance::StatusServer;


pub struct Timer<'a> {
	parent: &'a mut TimeKeeper,
	name: String,
	start: Instant
}

impl<'a> Drop for Timer<'a> {
	fn drop(&mut self) {
		let elapsed = Instant::now().duration_since(self.start);
		self.parent.add_time(&self.name, elapsed);
	}
}


pub struct TimeKeeper {
	times: HashMap<String, (Duration, u32)>
}

impl TimeKeeper {
	pub fn new() -> Self {
		Self {times: HashMap::new()}
	}

	fn add_time(&mut self, name: &str, elapsed: Duration) {
		if let Some(item) = self.times.get_mut(name) {
			(*item).0 += elapsed;
			(*item).1 += 1;
		} else {
			self.times.insert(name.to_string(), (elapsed, 1));
		}
	}

	pub fn get_timer(&mut self, name: &str) -> Timer {
		Timer {parent: self, name: name.to_string(), start: Instant::now()}
	}

	pub fn print(&mut self, status: &StatusServer) {
		let mut msg = String::new();
		for (name, (duration, count)) in &self.times {
			msg += &format!("{}: {} x {:?} each; {:?} total\n",
				name, count, *duration / *count, duration);
		}
		status.log_info(msg);
	}
}


pub fn debug_bytes(src: &[u8]) -> String {
	let mut dst = String::new();
	for &byte in src {
		if byte == b'\\' {
			dst += "\\\\";
		} else if byte >= 32 && byte < 127 {
			dst.push(byte as char);
		} else {
			dst += &format!("\\x{:0>2x}", byte);
		}
	}
	dst
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_debug_bytes() {
		let inp = b"\x00\x0a\x1f~~ Hello \\ World! ~~\x7f\xee\xff";
		let out = r"\x00\x0a\x1f~~ Hello \\ World! ~~\x7f\xee\xff";
		assert_eq!(&debug_bytes(&inp[..]), out);
	}
}
