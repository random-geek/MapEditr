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
		// This should be safe; EndInventory\n cannot appear in item metadata
		// since newlines are escaped.
		let end = end_finder
			.search_in(&data.get_ref()[data.position() as usize ..])
			.ok_or(MapBlockError::BadData)?;

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
			return Err(MapBlockError::InvalidSubVersion)
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


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_meta_serialize() {
		// Test empty metadata lists
		assert!(NodeMetadataList::deserialize(b"\x00").unwrap().is_empty());
		for &ver in &[25, 28] {
			assert_eq!(NodeMetadataList::new().serialize(ver), b"\x00");
		}

		// Test serialization/deserialization and filtering of empty metadata.
		let meta_in = b"\x02\x00\x04\
			\x00\x10\x00\x00\x00\x01\x00\x08formspec\x00\x00\x00\x24size[4,1]\
				list[context;main;0,0;4,1;]\x00List main 4\nWidth 0\nEmpty\n\
				Empty\nItem basenodes:cobble 1 0 \"\\u0001check\\u0002\
				EndInventory\\n\\u0003\"\nEmpty\nEndInventoryList\n\
				EndInventory\n\
			\x0e\x21\x00\x00\x00\x01\x00\x06secret\x00\x00\x00\x0a\x01pa55w0rd\
				\x02\x01EndInventory\n\
			\x03\x23\x00\x00\x00\x00EndInventory\n\
			\x0f\xff\x00\x00\x00\x00List main 1\nWidth 0\nItem basenodes:dirt_\
				with_grass 10\nEndInventoryList\nEndInventory\n";

		let meta_out = b"\x02\x00\x03\
			\x00\x10\x00\x00\x00\x01\x00\x08formspec\x00\x00\x00\x24size[4,1]\
				list[context;main;0,0;4,1;]\x00List main 4\nWidth 0\nEmpty\n\
				Empty\nItem basenodes:cobble 1 0 \"\\u0001check\\u0002\
				EndInventory\\n\\u0003\"\nEmpty\nEndInventoryList\n\
				EndInventory\n\
			\x0e\x21\x00\x00\x00\x01\x00\x06secret\x00\x00\x00\x0a\x01pa55w0rd\
				\x02\x01EndInventory\n\
			\x0f\xff\x00\x00\x00\x00List main 1\nWidth 0\nItem basenodes:dirt_\
				with_grass 10\nEndInventoryList\nEndInventory\n";

		let meta_list = NodeMetadataList::deserialize(&meta_in[..]).unwrap();
		assert_eq!(meta_list.len(), 4);
		assert_eq!(meta_list[&0x010].vars[&b"formspec"[..]].1, false);
		assert_eq!(meta_list[&0xe21].vars[&b"secret"[..]].1, true);
		assert_eq!(meta_list.serialize(28), meta_out);

		// Test currently unsupported version
		let mut meta_future = meta_in.to_vec();
		meta_future[0] = b'\x03';
		assert_eq!(
			NodeMetadataList::deserialize(&meta_future[..]).unwrap_err(),
			MapBlockError::InvalidSubVersion
		);

		// Test old version
		let meta_v1 = b"\x01\x00\x02\
			\x00\x10\x00\x00\x00\x01\x00\x08formspec\x00\x00\x00\x24size[4,1]\
				list[context;main;0,0;4,1;]List main 4\nWidth 0\nEmpty\n\
				Empty\nItem basenodes:cobble\nEmpty\nEndInventoryList\n\
				EndInventory\n\
			\x0d\xb7\x00\x00\x00\x00List main 1\nWidth 0\nItem basenodes:dirt_\
				with_grass 10\nEndInventoryList\nEndInventory\n";

		let meta_list_v1 =
			NodeMetadataList::deserialize(&meta_v1[..]).unwrap();
		assert_eq!(meta_list_v1.len(), 2);
		assert_eq!(meta_list_v1[&0x010].vars[&b"formspec"[..]].1, false);
		assert_eq!(meta_list_v1.serialize(25), meta_v1);

		// Test missing inventory
		let missing_inv = b"\x02\x00\x02\
			\x01\x23\x00\x00\x00\x01\
				\x00\x03foo\x00\x00\x00\x03bar\x00
			\x0f\xed\x00\x00\x00\x01\
				\x00\x0dfake_inv_test\x00\x00\x00\x0cEndInventory\x00";
		assert_eq!(NodeMetadataList::deserialize(missing_inv).unwrap_err(),
			MapBlockError::BadData);
	}
}
