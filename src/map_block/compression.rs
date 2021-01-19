use std::io::Cursor;
use std::io::prelude::*;

use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;

use super::MapBlockError;


pub trait Compress {
	fn compress(&self, dst: &mut Cursor<Vec<u8>>);
	fn decompress(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError>
		where Self: std::marker::Sized;
}


impl Compress for Vec<u8> {
	fn compress(&self, dst: &mut Cursor<Vec<u8>>) {
		let mut encoder = ZlibEncoder::new(dst, Compression::default());
		encoder.write_all(self.as_ref()).unwrap();
		encoder.finish().unwrap();
	}

	fn decompress(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError> {
		let start = src.position();

		let mut decoder = ZlibDecoder::new(src);
		let mut dst = Self::new();
		decoder.read_to_end(&mut dst).unwrap();
		let total_in = decoder.total_in();
		let src = decoder.into_inner();
		src.set_position(start + total_in);

		Ok(dst)
	}
}


#[derive(Debug)]
pub struct ZlibContainer<T: Compress> {
	compressed: Option<Vec<u8>>,
	data: T
}

impl<T: Compress> ZlibContainer<T> {
	pub fn read(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError> {
		let start = src.position() as usize;
		let data = T::decompress(src)?;
		let end = src.position() as usize;
		Ok(Self {
			compressed: Some(src.get_ref()[start..end].to_vec()),
			data
		})
	}

	pub fn write(&self, dst: &mut Cursor<Vec<u8>>) {
		if let Some(compressed) = self.compressed.as_deref() {
			dst.write_all(compressed).unwrap();
		} else {
			self.data.compress(dst);
		}
	}

	pub fn get_ref(&self) -> &T {
		&self.data
	}

	pub fn get_mut(&mut self) -> &mut T {
		self.compressed = None;
		&mut self.data
	}
}
