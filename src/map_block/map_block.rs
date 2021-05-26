use super::*;

/*
Supported mapblock versions:
25: In use from 0.4.2-rc1 until 0.4.15.
26: Only ever sent over the network, not saved.
27: Existed for around 3 months during 0.4.16 development.
28: In use since 0.4.16.
*/

const MIN_BLOCK_VER: u8 = 25;
const MAX_BLOCK_VER: u8 = 28;
const BLOCK_BUF_SIZE: usize = 2048;


pub fn is_valid_generated(data: &[u8]) -> bool {
	data.len() >= 2
		&& MIN_BLOCK_VER <= data[0] && data[0] <= MAX_BLOCK_VER
		&& data[1] & 0x08 == 0
}


#[derive(Clone, Debug)]
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
	pub fn deserialize(src: &[u8]) -> Result<Self, MapBlockError> {
		let mut data = Cursor::new(src);

		// Version
		let version = data.read_u8()?;
		if version < MIN_BLOCK_VER || version > MAX_BLOCK_VER {
			return Err(MapBlockError::InvalidBlockVersion);
		}

		// Flags
		let flags = data.read_u8()?;

		// Light data
		let lighting_complete =
			if version >= 27 { data.read_u16::<BigEndian>()? }
			else { 0xFFFF };

		// Content width/param width
		let content_width = data.read_u8()?;
		let params_width = data.read_u8()?;
		// TODO: support content_width == 1?
		if content_width != 2 || params_width != 2 {
			return Err(MapBlockError::InvalidFeature);
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
		// TODO: Retain compression level used by Minetest?
		// TODO: Use a bigger buffer (unsafe?) to reduce heap allocations.
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


#[cfg(test)]
mod tests {
	use super::*;
	use crate::spatial::Vec3;
	use std::path::Path;

	fn read_test_file(filename: &str) -> anyhow::Result<Vec<u8>> {
		let cargo_path = std::env::var("CARGO_MANIFEST_DIR")?;
		let path = Path::new(&cargo_path).join("testing").join(filename);
		Ok(std::fs::read(path)?)
	}

	#[test]
	fn test_mapblock_v28() {
		// Original block positioned at (0, 0, 0).
		let original_data = read_test_file("mapblock_v28.bin").unwrap();
		let block = MapBlock::deserialize(&original_data).unwrap();

		/* Ensure that all block data is correct. */
		assert_eq!(block.version, 28);
		assert_eq!(block.flags, 0x03);
		assert_eq!(block.lighting_complete, 0xF1C4);
		assert_eq!(block.content_width, 2);
		assert_eq!(block.params_width, 2);

		// Probe a few spots in the node data.
		let nd = block.node_data.get_ref();
		let test_node_id = block.nimap.get_id(b"test_mod:timer").unwrap();
		let air_id = block.nimap.get_id(b"air").unwrap();
		assert_eq!(nd.nodes[0x000], test_node_id);
		assert!(nd.nodes[0x001..=0xFFE].iter().all(|&n| n == air_id));
		assert_eq!(nd.nodes[0xFFF], test_node_id);
		assert_eq!(nd.param1[0x111], 0x0F);
		assert_eq!(nd.param2[0x000], 4);
		assert!(nd.param2[0x001..=0xFFE].iter().all(|&n| n == 0));
		assert_eq!(nd.param2[0xFFF], 16);

		assert_eq!(block.metadata.get_ref(), b"\x00");

		let obj1 = &block.static_objects[0];
		assert_eq!(obj1.obj_type, 7);
		assert_eq!(obj1.f_pos, Vec3::new(8, 9, 12) * 10_000);
		assert_eq!(obj1.data.len(), 62);
		let obj2 = &block.static_objects[1];
		assert_eq!(obj2.obj_type, 7);
		assert_eq!(obj2.f_pos, Vec3::new(1, 2, 2) * 10_000);
		assert_eq!(obj2.data.len(), 81);

		assert_eq!(block.timestamp, 2756);

		assert_eq!(block.nimap.0[&0], b"test_mod:timer");
		assert_eq!(block.nimap.0[&1], b"air");

		assert_eq!(block.node_timers[0].pos, 0xFFF);
		assert_eq!(block.node_timers[0].timeout, 1337);
		assert_eq!(block.node_timers[0].elapsed, 600);
		assert_eq!(block.node_timers[1].pos, 0x000);
		assert_eq!(block.node_timers[1].timeout, 1337);
		assert_eq!(block.node_timers[1].elapsed, 200);

		/* Test re-serialized data */
		let new_data = block.serialize();

		// If zlib-compressed data is reused, it should be identical.
		assert_eq!(new_data, original_data);

		// Triggering a data modification should change the compressed data,
		// since Minetest and MapEditr use different compression levels.
		let mut block2 = block.clone();
		block2.node_data.get_mut();
		assert_ne!(block2.serialize(), original_data);
	}

	#[test]
	fn test_mapblock_v25() {
		// Original block positioned at (-1, -1, -1).
		let original_data = read_test_file("mapblock_v25.bin").unwrap();
		let block = MapBlock::deserialize(&original_data).unwrap();

		/* Ensure that all block data is correct. */
		assert_eq!(block.version, 25);
		assert_eq!(block.flags, 0x03);
		assert_eq!(block.lighting_complete, 0xFFFF);
		assert_eq!(block.content_width, 2);
		assert_eq!(block.params_width, 2);

		let nd = block.node_data.get_ref();
		let test_node_id = block.nimap.get_id(b"test_mod:stone").unwrap();
		for z in &[0, 15] {
			for y in &[0, 15] {
				for x in &[0, 15] {
					assert_eq!(nd.nodes[x + 16 * (y + 16 * z)], test_node_id);
				}
			}
		}
		assert_eq!(nd.nodes[0x001], block.nimap.get_id(b"air").unwrap());
		assert_eq!(nd.nodes[0x111],
			block.nimap.get_id(b"test_mod:timer").unwrap());
		assert_eq!(nd.param2[0x111], 12);

		assert_eq!(block.metadata.get_ref(), b"\x00");

		let obj1 = &block.static_objects[0];
		assert_eq!(obj1.obj_type, 7);
		assert_eq!(obj1.f_pos, Vec3::new(-5, -10, -15) * 10_000);
		assert_eq!(obj1.data.len(), 72);

		let obj2 = &block.static_objects[1];
		assert_eq!(obj2.obj_type, 7);
		assert_eq!(obj2.f_pos, Vec3::new(-14, -12, -10) * 10_000);
		assert_eq!(obj2.data.len(), 54);

		assert_eq!(block.timestamp, 2529);

		assert_eq!(block.nimap.0[&0], b"test_mod:stone");
		assert_eq!(block.nimap.0[&1], b"air");
		assert_eq!(block.nimap.0[&2], b"test_mod:timer");

		assert_eq!(block.node_timers[0].pos, 0x111);
		assert_eq!(block.node_timers[0].timeout, 1337);
		assert_eq!(block.node_timers[0].elapsed, 0);

		/* Test re-serialized data */
		let mut block2 = block.clone();
		block2.node_data.get_mut();
		assert_ne!(block2.serialize(), original_data);
	}

	#[test]
	fn test_failures() {
		let data = read_test_file("mapblock_v28.bin").unwrap();

		// Change specific parts of the serialized data and make sure
		// MapBlock::deserialize() catches the errors. Something like a hex
		// editor is needed to follow along.

		let check_error =
			|modder: fn(&mut [u8]), expected_error: MapBlockError|
		{
			let mut copy = data.clone();
			modder(&mut copy);
			assert_eq!(MapBlock::deserialize(&copy).unwrap_err(),
				expected_error);
		};

		// Invalid versions
		check_error(|d| d[0x0] = 24, MapBlockError::InvalidBlockVersion);
		check_error(|d| d[0x0] = 29, MapBlockError::InvalidBlockVersion);
		// Invalid content width
		check_error(|d| d[0x4] = 1, MapBlockError::InvalidFeature);
		// Invalid parameter width
		check_error(|d| d[0x5] = 3, MapBlockError::InvalidFeature);
		// Invalid static object version
		check_error(|d| d[0xA9] = 1, MapBlockError::InvalidSubVersion);
		// Invalid name-ID map version
		check_error(|d| d[0x15D] = 1, MapBlockError::InvalidSubVersion);
		// Invalid node timer data length
		check_error(|d| d[0x179] = 12, MapBlockError::InvalidFeature);

		{ // Invalid node data size
			let mut block = MapBlock::deserialize(&data).unwrap();
			block.node_data.get_mut().param1.push(0);
			let new_data = block.serialize();
			assert_eq!(MapBlock::deserialize(&new_data).unwrap_err(),
				MapBlockError::BadData);

			block.node_data.get_mut().param1.truncate(4095);
			let new_data = block.serialize();
			assert_eq!(MapBlock::deserialize(&new_data).unwrap_err(),
				MapBlockError::BadData);
		}
	}
}
