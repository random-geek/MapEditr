use std::io::prelude::*;
use std::io::Cursor;
use byteorder::{ByteOrder, BigEndian, ReadBytesExt, WriteBytesExt};

mod map_block;
mod compression;
mod node_data;
mod metadata;
mod static_object;
mod node_timer;
mod name_id_map;

pub use map_block::{MapBlock, is_valid_generated};
pub use compression::ZlibContainer;
use compression::Compress;
pub use node_data::NodeData;
pub use metadata::NodeMetadataList;
pub use static_object::{StaticObject, StaticObjectList, LuaEntityData};
use static_object::{serialize_objects, deserialize_objects};
pub use node_timer::{NodeTimer, NodeTimerList};
use node_timer::{serialize_timers, deserialize_timers};
pub use name_id_map::NameIdMap;


#[derive(Clone, Debug)]
pub enum MapBlockError {
	InvalidVersion,
	DataError,
	Other,
}

impl From<std::io::Error> for MapBlockError {
	fn from(_: std::io::Error) -> Self {
		Self::DataError
	}
}


fn vec_with_len<T>(len: usize) -> Vec<T> {
	let mut v = Vec::with_capacity(len);
	unsafe { v.set_len(len) }
	v
}


fn read_string16(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, std::io::Error> {
	let count = src.read_u16::<BigEndian>()?;
	let mut bytes = vec_with_len(count as usize);
	src.read_exact(&mut bytes)?;
	Ok(bytes)
}


fn read_string32(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, std::io::Error> {
	let count = src.read_u32::<BigEndian>()?;
	let mut bytes = vec_with_len(count as usize);
	src.read_exact(&mut bytes)?;
	Ok(bytes)
}


fn write_string16(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	dst.write_u16::<BigEndian>(data.len() as u16).unwrap();
	dst.write(data).unwrap();
}


fn write_string32(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	dst.write_u32::<BigEndian>(data.len() as u32).unwrap();
	dst.write(data).unwrap();
}
