use super::*;
use std::cmp::min;


#[derive(Clone, Debug)]
pub struct NodeTimer {
	pub pos: u16,
	pub timeout: u32,
	pub elapsed: u32
}


pub type NodeTimerList = Vec<NodeTimer>;


pub fn deserialize_timers<T: Read>(src: &mut T)
	-> Result<NodeTimerList, MapBlockError>
{
	let data_len = src.read_u8()?;
	if data_len != 10 {
		return Err(MapBlockError::InvalidFeature);
	}

	let count = src.read_u16::<BigEndian>()?;
	// Limit allocation to number of nodes (bad data handling).
	let mut timers = Vec::with_capacity(min(count, 4096) as usize);

	for _ in 0..count {
		let pos = src.read_u16::<BigEndian>()?;
		let timeout = src.read_u32::<BigEndian>()?;
		let elapsed = src.read_u32::<BigEndian>()?;
		timers.push(NodeTimer {pos, timeout, elapsed});
	}

	Ok(timers)
}


pub fn serialize_timers<T: Write>(timers: &NodeTimerList, dst: &mut T) {
	dst.write_u8(10).unwrap();
	dst.write_u16::<BigEndian>(timers.len() as u16).unwrap();

	for t in timers {
		dst.write_u16::<BigEndian>(t.pos).unwrap();
		dst.write_u32::<BigEndian>(t.timeout).unwrap();
		dst.write_u32::<BigEndian>(t.elapsed).unwrap();
	}
}
