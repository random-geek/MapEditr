use std::collections::HashMap;

use memmem::{Searcher, TwoWaySearcher};

use super::*;


#[derive(Debug, Clone)]
pub struct NodeMetadata {
	pub vars: HashMap<Vec<u8>, (Vec<u8>, bool)>,
	pub inv: Vec<u8>
}

impl NodeMetadata {
	fn deserialize(data: &mut Cursor<&[u8]>, version: u8)
		-> Result<Self, MapBlockError>
	{
		let var_count = data.read_u32::<BigEndian>()?;
		let mut vars = HashMap::with_capacity(var_count as usize);

		for _ in 0..var_count {
			let name = read_string16(data)?;
			let val = read_string32(data)?;
			let private = if version >= 2 {
				data.read_u8()? != 0
			} else { false };
			vars.insert(name.clone(), (val, private));
		}

		const END_STR: &[u8; 13] = b"EndInventory\n";
		let end_finder = TwoWaySearcher::new(END_STR);
		let end = end_finder
			.search_in(&data.get_ref()[data.position() as usize ..])
			.ok_or(MapBlockError::Other)?;

		let mut inv = vec_with_len(end + END_STR.len());
		data.read_exact(&mut inv)?;

		Ok(Self {
			vars,
			inv
		})
	}

	fn serialize(&self, data: &mut Cursor<Vec<u8>>, version: u8) {
		data.write_u32::<BigEndian>(self.vars.len() as u32).unwrap();
		for (name, (val, private)) in &self.vars {
			write_string16(data, name);
			write_string32(data, &val);
			if version >= 2 {
				data.write_u8(*private as u8).unwrap();
			}
		}

		data.write_all(&self.inv).unwrap();
	}
}


#[derive(Debug)]
pub struct NodeMetadataList {
	// TODO: Switch to BTreeMap or something more stable
	// TODO: This is just a wrapper struct, switch to a type alias?
	pub list: HashMap<u16, NodeMetadata>
}

impl NodeMetadataList {
	pub fn deserialize(data_slice: &[u8]) -> Result<Self, MapBlockError> {
		let mut data = Cursor::new(data_slice);

		let version = data.read_u8()?;
		if version > 2 {
			return Err(MapBlockError::InvalidVersion)
		}

		let count = match version {
			0 => 0,
			_ => data.read_u16::<BigEndian>()?
		};

		let mut list = HashMap::with_capacity(count as usize);
		for _ in 0..count {
			let pos = data.read_u16::<BigEndian>()?;
			let meta = NodeMetadata::deserialize(&mut data, version)?;
			list.insert(pos, meta);
		}

		Ok(Self { list })
	}

	pub fn serialize(&self, block_version: u8) -> Vec<u8> {
		let buf = Vec::new();
		let mut data = Cursor::new(buf);

		if self.list.len() == 0 {
			data.write_u8(0).unwrap();
		} else {
			let version = if block_version >= 28 { 2 } else { 1 };
			data.write_u8(version).unwrap();
			data.write_u16::<BigEndian>(self.list.len() as u16).unwrap();

			for (&pos, meta) in &self.list {
				data.write_u16::<BigEndian>(pos).unwrap();
				meta.serialize(&mut data, version);
			}
		}

		data.into_inner()
	}
}
