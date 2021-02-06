use super::*;


#[derive(Debug, Clone)]
pub struct NodeTimer {
	pub pos: u16,
	pub timeout: u32,
	pub elapsed: u32
}


pub type NodeTimerList = Vec<NodeTimer>;


pub fn deserialize_timers(src: &mut Cursor<&[u8]>)
	-> Result<NodeTimerList, MapBlockError>
{
	let data_len = src.read_u8()?;
	if data_len != 10 {
		return Err(MapBlockError::Other);
	}

	let count = src.read_u16::<BigEndian>()?;
	let mut timers = Vec::with_capacity(count as usize);

	for _ in 0 .. count {
		let pos = src.read_u16::<BigEndian>()?;
		let timeout = src.read_u32::<BigEndian>()?;
		let elapsed = src.read_u32::<BigEndian>()?;
		timers.push(NodeTimer {pos, timeout, elapsed});
	}

	Ok(timers)
}


pub fn serialize_timers(timers: &NodeTimerList, dst: &mut Cursor<Vec<u8>>) {
	dst.write_u8(10).unwrap();
	dst.write_u16::<BigEndian>(timers.len() as u16).unwrap();

	for t in timers {
		dst.write_u16::<BigEndian>(t.pos).unwrap();
		dst.write_u32::<BigEndian>(t.timeout).unwrap();
		dst.write_u32::<BigEndian>(t.elapsed).unwrap();
	}
}
