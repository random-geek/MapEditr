use std::io::prelude::*;
use std::io::Cursor;
use std::convert::TryFrom;

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


#[derive(Clone, Debug, PartialEq)]
pub enum MapBlockError {
	/// Block data is malformed or missing.
	BadData,
	/// The block version is unsupported.
	InvalidBlockVersion,
	/// Some data length or other value is unsupported.
	InvalidFeature,
	/// Some content within the mapblock has an unsupported version.
	InvalidSubVersion,
}

impl From<std::io::Error> for MapBlockError {
	fn from(_: std::io::Error) -> Self {
		Self::BadData
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
	-> Result<Vec<u8>, MapBlockError>
{
	if src.get_ref().len() - (src.position() as usize) < n {
		// Corrupted length or otherwise not enough bytes to fill buffer.
		Err(MapBlockError::BadData)
	} else {
		let mut bytes = vec_with_len(n);
		src.read_exact(&mut bytes)?;
		Ok(bytes)
	}
}


fn read_string16(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, MapBlockError> {
	let count = src.read_u16::<BigEndian>()?;
	try_read_n(src, count as usize)
}


fn read_string32(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, MapBlockError> {
	let count = src.read_u32::<BigEndian>()?;
	try_read_n(src, count as usize)
}


fn write_string16(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	let len = u16::try_from(data.len()).unwrap();
	dst.write_u16::<BigEndian>(len).unwrap();
	dst.write(data).unwrap();
}


fn write_string32(dst: &mut Cursor<Vec<u8>>, data: &[u8]) {
	let len = u32::try_from(data.len()).unwrap();
	dst.write_u32::<BigEndian>(len).unwrap();
	dst.write(data).unwrap();
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[should_panic]
	fn test_string16_overflow() {
		let mut buf = Cursor::new(Vec::new());
		let long = (0..128).collect::<Vec<u8>>().repeat(512);
		write_string16(&mut buf, &long);
	}

	#[test]
	fn test_string_serialization() {
		let mut buf = Cursor::new(Vec::new());
		let long_string = b"lorem ipsum dolor sin amet ".repeat(10);
		let huge_string =
			b"There are only so many strings that have exactly 64 characters. "
			.repeat(1024);

		write_string16(&mut buf, b"");
		write_string16(&mut buf, &long_string);
		write_string32(&mut buf, b"");
		write_string32(&mut buf, &huge_string);

		let mut res = Vec::new();
		res.extend_from_slice(b"\x00\x00");
		res.extend_from_slice(b"\x01\x0E");
		res.extend_from_slice(&long_string);
		res.extend_from_slice(b"\x00\x00\x00\x00");
		res.extend_from_slice(b"\x00\x01\x00\x00");
		res.extend_from_slice(&huge_string);

		assert_eq!(buf.into_inner(), res);
	}

	#[test]
	fn test_string_deserialization() {
		let huge_string =
			b"Magic purple goats can eat up to 30 kg of purple hay every day. "
			.repeat(1024);

		let mut buf = Vec::new();
		buf.extend_from_slice(b"\x00\x00");
		buf.extend_from_slice(b"\x00\x0DHello, world!");
		buf.extend_from_slice(b"\x00\x01\x00\x00");
		buf.extend_from_slice(&huge_string);
		buf.extend_from_slice(b"\x00\x00\x00\x00");

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
		assert!(contains(read_string32(&mut cursor), &huge_string));
		assert!(contains(read_string32(&mut cursor), b""));

		let bad_string16s: &[&[u8]] = &[
			b"",
			b"\xFF",
			b"\x00\x01",
			b"\x00\x2D actual data length < specified data length!",
		];
		for &bad in bad_string16s {
			assert_eq!(read_string16(&mut Cursor::new(&bad)),
				Err(MapBlockError::BadData));
		}

		let bad_string32s: &[&[u8]] = &[
			b"",
			b"\x00\x00",
			b"\x00\x00\x00\x01",
			b"\xFF\xFF\xFF\xFF",
			b"\x00\x00\x00\x2D actual data length < specified data length!",
		];
		for &bad in bad_string32s {
			assert_eq!(read_string32(&mut Cursor::new(&bad)),
				Err(MapBlockError::BadData));
		}
	}
}
