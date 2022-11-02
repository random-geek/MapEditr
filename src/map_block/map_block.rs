use super::*;
use zstd;

/*
Supported mapblock versions:
25: In use from 0.4.2-rc1 until 0.4.15.
26: Only ever sent over the network, not saved.
27: Existed for around 3 months during 0.4.16 development.
28: In use from 0.4.16 to 5.4.x.
29: In use since 5.5.0 (mapblocks are now compressed with zstd instead of zlib).
*/

const MIN_BLOCK_VER: u8 = 25;
const MAX_BLOCK_VER: u8 = 29;
const SERIALIZE_BUF_SIZE: usize = 2048;


pub fn is_valid_generated(src: &[u8]) -> bool {
	if src.len() < 2 {
		return false;
	}

	let mut crs = Cursor::new(src);
	let version = crs.read_u8().unwrap();
	if version < MIN_BLOCK_VER || version > MAX_BLOCK_VER {
		return false;
	}

	let flags = if version >= 29 {
		let mut dec = zstd::stream::Decoder::new(crs).unwrap();
		match dec.read_u8() {
			Ok(f) => f,
			Err(_) => return false
		}
	} else {
		crs.read_u8().unwrap()
	};

	flags & 0x08 == 0 // Bit 3 set if block is not generated.
}


#[derive(Clone, Debug)]
pub struct MapBlock {
	pub version: u8,
	pub flags: u8,
	pub lighting_complete: u16,
	pub content_width: u8,
	pub params_width: u8,
	pub node_data: NodeData,
	pub metadata: NodeMetadataList,
	pub static_objects: StaticObjectList,
	pub timestamp: u32,
	pub nimap: NameIdMap,
	pub node_timers: NodeTimerList
}

impl MapBlock {
	pub fn deserialize(src: &[u8]) -> Result<Self, MapBlockError> {
		let mut raw_crs = Cursor::new(src);

		let version = raw_crs.read_u8()?;
		if version < MIN_BLOCK_VER || version > MAX_BLOCK_VER {
			return Err(MapBlockError::InvalidBlockVersion);
		}

		// TODO: use thread_local buffer for decompressed data.
		let decompressed;
		let mut crs =
			if version >= 29 {
				decompressed = zstd::stream::decode_all(raw_crs)?;
				Cursor::new(decompressed.as_slice())
			} else { raw_crs };

		let flags = crs.read_u8()?;
		let lighting_complete =
			if version >= 27 { crs.read_u16::<BigEndian>()? }
			else { 0xFFFF };

		let mut timestamp = 0;
		let mut nimap = None; // Use Option to avoid re-initializing the map.

		if version >= 29 { // Timestamp/Name-ID map were moved in v29.
			timestamp = crs.read_u32::<BigEndian>()?;
			nimap = Some(NameIdMap::deserialize(&mut crs)?);
		}

		let content_width = crs.read_u8()?;
		let params_width = crs.read_u8()?;
		// TODO: support content_width == 1?
		if content_width != 2 || params_width != 2 {
			return Err(MapBlockError::InvalidFeature);
		}

		let node_data =
			if version >= 29 {
				NodeData::deserialize(&mut crs)?
			} else {
				NodeData::decompress(&mut crs)?
			};

		let metadata =
			if version >= 29 {
				NodeMetadataList::deserialize(&mut crs)?
			} else {
				NodeMetadataList::decompress(&mut crs)?
			};

		let static_objects = deserialize_objects(&mut crs)?;

		if version < 29 {
			timestamp = crs.read_u32::<BigEndian>()?;
			nimap = Some(NameIdMap::deserialize(&mut crs)?);
		}

		let node_timers = deserialize_timers(&mut crs)?;

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
			nimap: nimap.unwrap(),
			node_timers
		})
	}

	pub fn serialize(&self) -> Vec<u8> {
		// TODO: Retain compression level used by Minetest?
		assert!(MIN_BLOCK_VER <= self.version && self.version <= MAX_BLOCK_VER,
			"Invalid mapblock version.");

		// TODO: Use a bigger buffer (unsafe?) to reduce heap allocations.
		let mut buf = Vec::with_capacity(SERIALIZE_BUF_SIZE);
		let mut crs = Cursor::new(buf);
		crs.write_u8(self.version).unwrap();

		if self.version >= 29 {
			let mut enc = zstd::stream::Encoder::new(crs, 0).unwrap();

			enc.write_u8(self.flags).unwrap();
			enc.write_u16::<BigEndian>(self.lighting_complete).unwrap();
			enc.write_u32::<BigEndian>(self.timestamp).unwrap();
			self.nimap.serialize(&mut enc);
			enc.write_u8(self.content_width).unwrap();
			enc.write_u8(self.params_width).unwrap();
			self.node_data.serialize(&mut enc);
			self.metadata.serialize(&mut enc, self.version);
			serialize_objects(&self.static_objects, &mut enc);
			serialize_timers(&self.node_timers, &mut enc);

			crs = enc.finish().unwrap();
		} else { // version <= 28
			crs.write_u8(self.flags).unwrap();

			if self.version >= 27 {
				crs.write_u16::<BigEndian>(self.lighting_complete).unwrap();
			}

			crs.write_u8(self.content_width).unwrap();
			crs.write_u8(self.params_width).unwrap();
			self.node_data.compress(&mut crs);
			self.metadata.compress(&mut crs, self.version);
			serialize_objects(&self.static_objects, &mut crs);
			crs.write_u32::<BigEndian>(self.timestamp).unwrap();
			self.nimap.serialize(&mut crs);
			serialize_timers(&self.node_timers, &mut crs);
		}

		buf = crs.into_inner();
		buf.shrink_to_fit();
		buf
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::spatial::Vec3;
	use std::path::Path;

	#[test]
	fn test_is_valid_generated() {
		let ivg = is_valid_generated;

		// Too short
		assert_eq!(ivg(b""), false);
		assert_eq!(ivg(b"\x18"), false); // v24
		assert_eq!(ivg(b"\x1D"), false); // v29
		// Invalid version
		assert_eq!(ivg(b"\x18\x00\x00\x00"), false); // v24
		assert_eq!(ivg(b"\x1E\x00\x00\x00"), false); // v30
		// v28, "not generated" flag set
		assert_eq!(ivg(b"\x1C\x08"), false);
		// v29, zstd compressed data is unreadable
		assert_eq!(ivg(b"\x1D\x00\xFF"), false);
		// v29, "not generated" flag set
		assert_eq!(ivg(b"\x1D\x28\xB5\x2F\xFD\x00\x58\x19\x00\x00\x08\xFF\xFF"), false);
		// v28, good
		assert_eq!(ivg(b"\x1C\x00"), true);
		// v29, good
		assert_eq!(ivg(b"\x1D\x28\xB5\x2F\xFD\x00\x58\x19\x00\x00\x00\xFF\xFF"), true);
	}

	fn read_test_file(filename: &str) -> anyhow::Result<Vec<u8>> {
		let cargo_path = std::env::var("CARGO_MANIFEST_DIR")?;
		let path = Path::new(&cargo_path).join("testing").join(filename);
		Ok(std::fs::read(path)?)
	}

	#[test]
	fn test_mapblock_v29() {
		// Original block positioned at (0, 0, 0).
		let data1 = read_test_file("mapblock_v29.bin").unwrap();
		let block1 = MapBlock::deserialize(&data1).unwrap();
		// Re-serialize and re-deserialize to test serialization, since
		// serialization results can vary.
		let data2 = block1.serialize();
		let block2 = MapBlock::deserialize(&data2).unwrap();

		for block in &[block1, block2] {
			/* Ensure that all block data is correct. */
			assert_eq!(block.version, 29);
			assert_eq!(block.flags, 0x03);
			assert_eq!(block.lighting_complete, 0xFFFF);
			assert_eq!(block.content_width, 2);
			assert_eq!(block.params_width, 2);

			// Probe a few spots in the node data.
			let nd = &block.node_data;
			let timer_node_id = block.nimap.get_id(b"test_mod:timer").unwrap();
			let meta_node_id = block.nimap.get_id(b"test_mod:metadata").unwrap();
			let air_id = block.nimap.get_id(b"air").unwrap();
			assert_eq!(nd.nodes[0x000], timer_node_id);
			assert!(nd.nodes[0x001..=0xFFE].iter().all(|&n| n == air_id));
			assert_eq!(nd.nodes[0xFFF], meta_node_id);
			assert_eq!(nd.param2[0x000], 19);
			assert_eq!(nd.param1[0x111], 0x0F);
			assert!(nd.param2[0x001..=0xFFF].iter().all(|&n| n == 0));

			assert_eq!(block.metadata.len(), 1);
			let meta = &block.metadata[&4095];
			assert_eq!(meta.vars.len(), 2);
			let formspec_var = meta.vars.get(&b"formspec".to_vec()).unwrap();
			assert_eq!(formspec_var.0.len(), 75);
			assert_eq!(formspec_var.1, false);
			let infotext_var = meta.vars.get(&b"infotext".to_vec()).unwrap();
			assert_eq!(infotext_var.0, b"Test Chest");
			assert_eq!(infotext_var.1, false);
			assert_eq!(meta.inv.len(), 70);

			let obj1 = &block.static_objects[0];
			assert_eq!(obj1.obj_type, 7);
			assert_eq!(obj1.f_pos, Vec3::new(1, 2, 2) * 10_000);
			assert_eq!(obj1.data.len(), 75);
			let obj2 = &block.static_objects[1];
			assert_eq!(obj2.obj_type, 7);
			assert_eq!(obj2.f_pos, Vec3::new(8, 9, 12) * 10_000);
			assert_eq!(obj2.data.len(), 62);

			assert_eq!(block.timestamp, 542);

			assert_eq!(block.nimap.0[&0], b"test_mod:timer");
			assert_eq!(block.nimap.0[&1], b"air");

			assert_eq!(block.node_timers[0].pos, 0x000);
			assert_eq!(block.node_timers[0].timeout, 1337);
			assert_eq!(block.node_timers[0].elapsed, 399);
		}
	}

	#[test]
	fn test_mapblock_v28() {
		// Original block positioned at (0, 0, 0).
		let data1 = read_test_file("mapblock_v28.bin").unwrap();
		let block1 = MapBlock::deserialize(&data1).unwrap();
		let data2 = block1.serialize();
		let block2 = MapBlock::deserialize(&data2).unwrap();

		for block in &[block1, block2] {
			/* Ensure that all block data is correct. */
			assert_eq!(block.version, 28);
			assert_eq!(block.flags, 0x03);
			assert_eq!(block.lighting_complete, 0xF1C4);
			assert_eq!(block.content_width, 2);
			assert_eq!(block.params_width, 2);

			// Probe a few spots in the node data.
			let nd = &block.node_data;
			let test_node_id = block.nimap.get_id(b"test_mod:timer").unwrap();
			let air_id = block.nimap.get_id(b"air").unwrap();
			assert_eq!(nd.nodes[0x000], test_node_id);
			assert!(nd.nodes[0x001..=0xFFE].iter().all(|&n| n == air_id));
			assert_eq!(nd.nodes[0xFFF], test_node_id);
			assert_eq!(nd.param1[0x111], 0x0F);
			assert_eq!(nd.param2[0x000], 4);
			assert!(nd.param2[0x001..=0xFFE].iter().all(|&n| n == 0));
			assert_eq!(nd.param2[0xFFF], 16);

			assert!(block.metadata.is_empty());

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
		}
	}

	#[test]
	fn test_mapblock_v25() {
		// Original block positioned at (-1, -1, -1).
		let data1 = read_test_file("mapblock_v25.bin").unwrap();
		let block1 = MapBlock::deserialize(&data1).unwrap();
		let data2 = block1.serialize();
		let block2 = MapBlock::deserialize(&data2).unwrap();

		for block in &[block1, block2] {
			/* Ensure that all block data is correct. */
			assert_eq!(block.version, 25);
			assert_eq!(block.flags, 0x03);
			assert_eq!(block.lighting_complete, 0xFFFF);
			assert_eq!(block.content_width, 2);
			assert_eq!(block.params_width, 2);

			let nd = &block.node_data;
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

			assert!(block.metadata.is_empty());

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
		}
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
		check_error(|d| d[0x0] = 30, MapBlockError::InvalidBlockVersion);
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
			block.node_data.param1.push(0);
			let new_data = block.serialize();
			assert_eq!(MapBlock::deserialize(&new_data).unwrap_err(),
				MapBlockError::BadData);

			block.node_data.param1.truncate(4095);
			let new_data = block.serialize();
			assert_eq!(MapBlock::deserialize(&new_data).unwrap_err(),
				MapBlockError::BadData);
		}
	}
}
