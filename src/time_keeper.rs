use std::collections::HashMap;
use std::time::{Instant, Duration};


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

	/*pub fn print(&mut self) {
		println!("");
		for (name, (duration, count)) in &self.times {
			println!("{}: {} x {:?} each; {:?} total",
				name, count, *duration / *count, duration);
		}
	}*/
}
