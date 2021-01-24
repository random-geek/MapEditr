use super::*;

const MIN_BLOCK_VER: u8 = 25;
const MAX_BLOCK_VER: u8 = 28;
const BLOCK_BUF_SIZE: usize = 2048;


pub fn is_valid_generated(data: &[u8]) -> bool {
	data.len() > 2
		&& MIN_BLOCK_VER <= data[0] && data[0] <= MAX_BLOCK_VER
		&& data[1] & 0x08 == 0
}


#[derive(Debug)]
pub struct MapBlock {
	pub version: u8,
	pub flags: u8,
	pub lighting_complete: u16,
	pub content_width: u8,
	pub params_width: u8,
	pub node_data: ZlibContainer<NodeData>,
	pub metadata: ZlibContainer<Vec<u8>>,
	pub static_objects: StaticObjectList,
	pub timestamp: u32,
	pub nimap: NameIdMap,
	pub node_timers: NodeTimerList
}

impl MapBlock {
	pub fn deserialize(data_slice: &[u8]) -> Result<Self, MapBlockError> {
		let mut data = Cursor::new(data_slice);

		// Version
		let version = data.read_u8()?;
		if version < MIN_BLOCK_VER || version > MAX_BLOCK_VER {
			return Err(MapBlockError::InvalidVersion);
		}

		// Flags
		let flags = data.read_u8()?;

		// Light data
		let lighting_complete =
			if version >= 27 { data.read_u16::<BigEndian>()? }
			else { 0 };

		// Content width/param width
		let content_width = data.read_u8()?;
		let params_width = data.read_u8()?;
		if content_width != 2 || params_width != 2 {
			return Err(MapBlockError::Other);
		}

		// Node data
		let node_data = ZlibContainer::read(&mut data)?;
		// Node metadata
		let metadata = ZlibContainer::read(&mut data)?;
		// Static objects
		let static_objects = deserialize_objects(&mut data)?;
		// Timestamp
		let timestamp = data.read_u32::<BigEndian>()?;
		// Name-ID mappings
		let nimap = NameIdMap::deserialize(&mut data)?;
		// Node timers
		let node_timers = deserialize_timers(&mut data)?;

		Ok(Self {
			version,
			flags,
			lighting_complete,
			content_width,
			params_width,
			node_data,
			metadata,
			static_objects,
			timestamp,
			nimap,
			node_timers
		})
	}

	pub fn serialize(&self) -> Vec<u8> {
		let mut buf = Vec::with_capacity(BLOCK_BUF_SIZE);
		let mut data = Cursor::new(buf);

		assert!(MIN_BLOCK_VER <= self.version && self.version <= MAX_BLOCK_VER,
			"Invalid mapblock version.");

		// Version
		data.write_u8(self.version).unwrap();
		// Flags
		data.write_u8(self.flags).unwrap();

		// Light data
		if self.version >= 27 {
			data.write_u16::<BigEndian>(self.lighting_complete).unwrap();
		}

		// Content width/param width
		data.write_u8(self.content_width).unwrap();
		data.write_u8(self.params_width).unwrap();

		// Node data
		self.node_data.write(&mut data);
		// Node metadata
		self.metadata.write(&mut data);
		// Static objects
		serialize_objects(&self.static_objects, &mut data);
		// Timestamp
		data.write_u32::<BigEndian>(self.timestamp).unwrap();
		// Name-ID mappings
		self.nimap.serialize(&mut data);
		// Node timers
		serialize_timers(&self.node_timers, &mut data);

		buf = data.into_inner();
		buf.shrink_to_fit();
		buf
	}
}
