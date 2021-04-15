use super::*;
use std::collections::BTreeMap;


/// Maps 16-bit node IDs to actual node names.
///
/// Relevant Minetest source file: /src/nameidmapping.cpp
#[derive(Clone, Debug)]
pub struct NameIdMap(pub BTreeMap<u16, Vec<u8>>);

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

		for _ in 0..count {
			let id = data.read_u16::<BigEndian>()?;
			let name = read_string16(data)?;
			map.insert(id, name);
		}

		Ok(Self(map))
	}

	pub fn serialize(&self, out: &mut Cursor<Vec<u8>>) {
		out.write_u8(0).unwrap();
		out.write_u16::<BigEndian>(self.0.len() as u16).unwrap();

		for (id, name) in &self.0 {
			out.write_u16::<BigEndian>(*id).unwrap();
			write_string16(out, name);
		}
	}

	#[inline]
	pub fn get_id(&self, name: &[u8]) -> Option<u16> {
		self.0.iter().find_map(|(&k, v)|
			if v.as_slice() == name { Some(k) } else { None }
		)
	}

	#[inline]
	pub fn get_max_id(&self) -> Option<u16> {
		self.0.iter().next_back().map(|(&k, _)| k)
	}

	/// Remove the name at a given ID and shift down any values above it.
	pub fn remove_shift(&mut self, id: u16) {
		self.0.remove(&id);
		for k in id + 1 ..= self.get_max_id().unwrap_or(0) {
			if let Some(name) = self.0.remove(&k) {
				self.0.insert(k - 1, name);
			}
		}
	}
}
