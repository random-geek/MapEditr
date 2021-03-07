use super::*;
use crate::spatial::Vec3;
use std::cmp::min;


#[derive(Clone, Debug)]
pub struct StaticObject {
	pub obj_type: u8,
	pub f_pos: Vec3,
	pub data: Vec<u8>
}

impl StaticObject {
	fn deserialize(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError> {
		let obj_type = src.read_u8()?;
		let f_pos = Vec3::new(
			src.read_i32::<BigEndian>()?,
			src.read_i32::<BigEndian>()?,
			src.read_i32::<BigEndian>()?
		);
		let data = read_string16(src)?;
		Ok(Self {obj_type, f_pos, data})
	}

	fn serialize(&self, dst: &mut Cursor<Vec<u8>>) {
		dst.write_u8(self.obj_type).unwrap();
		dst.write_i32::<BigEndian>(self.f_pos.x).unwrap();
		dst.write_i32::<BigEndian>(self.f_pos.y).unwrap();
		dst.write_i32::<BigEndian>(self.f_pos.z).unwrap();
		write_string16(dst, &self.data);
	}
}


pub type StaticObjectList = Vec<StaticObject>;


pub fn deserialize_objects(src: &mut Cursor<&[u8]>)
	-> Result<StaticObjectList, MapBlockError>
{
	let version = src.read_u8()?;
	if version != 0 {
		return Err(MapBlockError::Other);
	}

	let count = src.read_u16::<BigEndian>()?;
	// Limit allocation to MT's default max object count (bad data handling).
	let mut list = Vec::with_capacity(min(count, 64) as usize);
	for _ in 0..count {
		list.push(StaticObject::deserialize(src)?);
	}

	Ok(list)
}


pub fn serialize_objects(objects: &StaticObjectList, dst: &mut Cursor<Vec<u8>>)
{
	dst.write_u8(0).unwrap();
	dst.write_u16::<BigEndian>(objects.len() as u16).unwrap();
	for obj in objects {
		obj.serialize(dst);
	}
}


pub struct LuaEntityData {
	pub name: Vec<u8>,
	pub data: Vec<u8>
}

impl LuaEntityData {
	pub fn deserialize(src: &StaticObject) -> Result<Self, MapBlockError> {
		if src.obj_type != 7 {
			return Err(MapBlockError::Other);
		}
		let mut src_data = Cursor::new(src.data.as_slice());
		if src_data.read_u8()? != 1 {
			return Err(MapBlockError::Other);
		}

		let name = read_string16(&mut src_data)?;
		let data = read_string32(&mut src_data)?;
		Ok(Self {name, data})
	}
}
