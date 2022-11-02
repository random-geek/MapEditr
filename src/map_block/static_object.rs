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

	fn serialize<T: Write>(&self, dst: &mut T) {
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
		return Err(MapBlockError::InvalidSubVersion);
	}

	let count = src.read_u16::<BigEndian>()?;
	// Limit allocation to MT's default max object count (bad data handling).
	let mut list = Vec::with_capacity(min(count, 64) as usize);
	for _ in 0..count {
		list.push(StaticObject::deserialize(src)?);
	}

	Ok(list)
}


pub fn serialize_objects<T: Write>(objects: &StaticObjectList, dst: &mut T)
{
	dst.write_u8(0).unwrap();
	dst.write_u16::<BigEndian>(objects.len() as u16).unwrap();
	for obj in objects {
		obj.serialize(dst);
	}
}


/// Stores the name and data of a LuaEntity (Minetest's standard entity type).
///
/// Relevant Minetest source file: src/server/luaentity_sao.cpp
#[derive(Debug)]
pub struct LuaEntityData {
	pub name: Vec<u8>,
	pub data: Vec<u8>
}

impl LuaEntityData {
	pub fn deserialize(src: &StaticObject) -> Result<Self, MapBlockError> {
		if src.obj_type != 7 {
			return Err(MapBlockError::InvalidFeature);
		}
		let mut src_data = Cursor::new(src.data.as_slice());
		if src_data.read_u8()? != 1 {
			// Unsupported LuaEntity version
			return Err(MapBlockError::InvalidSubVersion);
		}

		let name = read_string16(&mut src_data)?;
		let data = read_string32(&mut src_data)?;
		Ok(Self {name, data})
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_lua_entity() {
		let test_obj = StaticObject {
			obj_type: 7,
			f_pos: Vec3::new(4380, 17279, 32630),
			data: b"\x01\x00\x0e__builtin:item\x00\x00\x00\x6e\
				return {[\"age\"] = 0.91899997927248478, \
				[\"itemstring\"] = \"basenodes:cobble 2\", \
				[\"dropped_by\"] = \"singleplayer\"}\
				\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
				\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00".to_vec()
		};
		let entity = LuaEntityData::deserialize(&test_obj).unwrap();
		assert_eq!(entity.name, b"__builtin:item");
		assert_eq!(entity.data,
			b"return {[\"age\"] = 0.91899997927248478, \
			[\"itemstring\"] = \"basenodes:cobble 2\", \
			[\"dropped_by\"] = \"singleplayer\"}");

		let mut wrong_version = test_obj.clone();
		wrong_version.data[0] = 0;
		assert_eq!(LuaEntityData::deserialize(&wrong_version).unwrap_err(),
			MapBlockError::InvalidSubVersion);

		let wrong_type = StaticObject { obj_type: 6, ..test_obj };
		assert_eq!(LuaEntityData::deserialize(&wrong_type).unwrap_err(),
			MapBlockError::InvalidFeature);
	}
}
