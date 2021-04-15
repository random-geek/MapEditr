use super::*;

use std::collections::{HashMap, BTreeMap};
use std::cmp::min;

use memmem::{Searcher, TwoWaySearcher};


const END_STR: &[u8; 13] = b"EndInventory\n";


#[derive(Clone, Debug)]
pub struct NodeMetadata {
	pub vars: HashMap<Vec<u8>, (Vec<u8>, bool)>,
	pub inv: Vec<u8>
}

impl NodeMetadata {
	fn deserialize(data: &mut Cursor<&[u8]>, version: u8)
		-> Result<Self, MapBlockError>
	{
		let var_count = data.read_u32::<BigEndian>()?;
		// Avoid allocating huge numbers of variables (bad data handling).
		let mut vars = HashMap::with_capacity(min(var_count as usize, 64));

		for _ in 0..var_count {
			let name = read_string16(data)?;
			let val = read_string32(data)?;
			let private = if version >= 2 {
				data.read_u8()? != 0
			} else { false };
			vars.insert(name.clone(), (val, private));
		}

		let end_finder = TwoWaySearcher::new(END_STR);
		let end = end_finder
			.search_in(&data.get_ref()[data.position() as usize ..])
			.ok_or(MapBlockError::Other)?;

		let mut inv = vec_with_len(end + END_STR.len());
		data.read_exact(&mut inv)?;

		Ok(Self { vars, inv })
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

	/// Return `true` if the metadata contains no variables or inventory lists.
	fn is_empty(&self) -> bool {
		self.vars.is_empty() && self.inv.starts_with(END_STR)
	}
}


pub trait NodeMetadataListExt {
	fn deserialize(src: &[u8]) -> Result<Self, MapBlockError>
		where Self: std::marker::Sized;
	fn serialize(&self, block_version: u8) -> Vec<u8>;
}


pub type NodeMetadataList = BTreeMap<u16, NodeMetadata>;

impl NodeMetadataListExt for NodeMetadataList {
	fn deserialize(src: &[u8]) -> Result<Self, MapBlockError> {
		let mut data = Cursor::new(src);

		let version = data.read_u8()?;
		if version > 2 {
			return Err(MapBlockError::InvalidVersion)
		}

		let count = match version {
			0 => 0,
			_ => data.read_u16::<BigEndian>()?
		};

		let mut list = BTreeMap::new();
		for _ in 0..count {
			let pos = data.read_u16::<BigEndian>()?;
			let meta = NodeMetadata::deserialize(&mut data, version)?;
			list.insert(pos, meta);
		}

		Ok(list)
	}

	fn serialize(&self, block_version: u8) -> Vec<u8> {
		let buf = Vec::new();
		let mut data = Cursor::new(buf);
		// Skip empty metadata when serializing.
		let count = self.iter().filter(|&(_, m)| !m.is_empty()).count();

		if count == 0 {
			data.write_u8(0).unwrap();
		} else {
			let version = if block_version >= 28 { 2 } else { 1 };
			data.write_u8(version).unwrap();
			data.write_u16::<BigEndian>(count as u16).unwrap();

			for (&pos, meta) in self {
				if !meta.is_empty() {
					data.write_u16::<BigEndian>(pos).unwrap();
					meta.serialize(&mut data, version);
				}
			}
		}

		data.into_inner()
	}
}
