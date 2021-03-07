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
pub use metadata::{NodeMetadataList, NodeMetadataListExt};
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


/// Return `n` bytes of data from `src`. Will fail safely if there are not
/// enough bytes in `src`.
#[inline(always)]
fn try_read_n(src: &mut Cursor<&[u8]>, n: usize)
	-> Result<Vec<u8>, std::io::Error>
{
	if src.get_ref().len() - (src.position() as usize) < n {
		Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof,
			"not enough bytes to fill buffer"))
	} else {
		let mut bytes = vec_with_len(n);
		src.read_exact(&mut bytes)?;
		Ok(bytes)
	}
}


fn read_string16(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, std::io::Error> {
	let count = src.read_u16::<BigEndian>()?;
	try_read_n(src, count as usize)
}


fn read_string32(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, std::io::Error> {
	let count = src.read_u32::<BigEndian>()?;
	try_read_n(src, count as usize)
}


fn write_string16(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	dst.write_u16::<BigEndian>(data.len() as u16).unwrap();
	dst.write(data).unwrap();
}


fn write_string32(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	dst.write_u32::<BigEndian>(data.len() as u32).unwrap();
	dst.write(data).unwrap();
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_string_serialization() {
		let buf =
			b"\x00\x00\
			\x00\x0DHello, world!\
			\x00\x00\x00\x10more test data..\
			\x00\x00\x00\x00\
			\x00\x00\x00\x11corrupted length";
		let mut cursor = Cursor::new(&buf[..]);

		fn contains<E>(res: Result<Vec<u8>, E>, val: &[u8]) -> bool {
			if let Ok(inner) = res {
				inner == val
			} else {
				false
			}
		}

		assert!(contains(read_string16(&mut cursor), b""));
		assert!(contains(read_string16(&mut cursor), b"Hello, world!"));
		assert!(contains(read_string32(&mut cursor), b"more test data.."));
		assert!(contains(read_string32(&mut cursor), b""));
		assert!(read_string32(&mut cursor).is_err());
	}
}
