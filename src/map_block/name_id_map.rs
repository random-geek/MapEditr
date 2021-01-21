use std::collections::BTreeMap;

use super::*;


/// Maps 16-bit node IDs to actual node names.
/// Relevant Minetest source file: /src/nameidmapping.cpp
#[derive(Debug)]
pub struct NameIdMap {
	// Use a BTreeMap instead of a HashMap to preserve the order of IDs.
	pub map: BTreeMap<u16, Vec<u8>>,
}

impl NameIdMap {
	pub fn deserialize(data: &mut Cursor<&[u8]>)
		-> Result<Self, MapBlockError>
	{
		let version = data.read_u8()?;
		if version != 0 {
			return Err(MapBlockError::Other);
		}

		let count = data.read_u16::<BigEndian>()? as usize;
		let mut map = BTreeMap::new();

		for _ in 0 .. count {
			let id = data.read_u16::<BigEndian>()?;
			let name = read_string16(data)?;
			map.insert(id, name);
		}

		Ok(Self {map})
	}

	pub fn serialize(&self, out: &mut Cursor<Vec<u8>>) {
		out.write_u8(0).unwrap();
		out.write_u16::<BigEndian>(self.map.len() as u16).unwrap();

		for (id, name) in &self.map {
			out.write_u16::<BigEndian>(*id).unwrap();
			write_string16(out, name);
		}
	}

	#[inline]
	pub fn get_id(&self, name: &[u8]) -> Option<u16> {
		self.map.iter().find_map(|(&k, v)|
			if v.as_slice() == name { Some(k) } else { None }
		)
	}

	#[inline]
	pub fn get_max_id(&self) -> Option<u16> {
		self.map.iter().next_back().map(|k| *(k.0))
	}

	#[inline]
	pub fn insert(&mut self, id: u16, name: &[u8]) {
		self.map.insert(id, name.to_owned());
	}

	/// Remove the name at a given ID and shift down values above it.
	pub fn remove(&mut self, id: u16) {
		self.map.remove(&id);
		let mut next_id = id + 1;

		while self.map.contains_key(&next_id) {
			let name = self.map.remove(&next_id).unwrap();
			self.map.insert(next_id - 1, name);
			next_id += 1;
		}
	}
}
