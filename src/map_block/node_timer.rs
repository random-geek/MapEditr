use super::*;


#[derive(Debug)]
pub struct NodeTimer {
	pos: u16,
	timeout: u32,
	elapsed: u32
}


#[derive(Debug)]
pub struct NodeTimerList {
	timers: Vec<NodeTimer>
}

impl NodeTimerList {
	pub fn deserialize(data: &mut Cursor<&[u8]>)
		-> Result<Self, MapBlockError>
	{
		let data_len = data.read_u8()?;
		if data_len != 10 {
			return Err(MapBlockError::Other);
		}

		let count = data.read_u16::<BigEndian>()?;
		let mut timers = Vec::with_capacity(count as usize);

		for _ in 0 .. count {
			let pos = data.read_u16::<BigEndian>()?;
			let timeout = data.read_u32::<BigEndian>()?;
			let elapsed = data.read_u32::<BigEndian>()?;
			timers.push(NodeTimer {pos, timeout, elapsed});
		}

		Ok(NodeTimerList {timers})
	}

	pub fn serialize(&self, data: &mut Cursor<Vec<u8>>) {
		data.write_u8(10).unwrap();
		data.write_u16::<BigEndian>(self.timers.len() as u16).unwrap();

		for t in &self.timers {
			data.write_u16::<BigEndian>(t.pos).unwrap();
			data.write_u32::<BigEndian>(t.timeout).unwrap();
			data.write_u32::<BigEndian>(t.elapsed).unwrap();
		}
	}
}
