use super::*;

use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;


const BLOCK_SIZE: usize = 16;
const NODE_COUNT: usize = BLOCK_SIZE * BLOCK_SIZE * BLOCK_SIZE;


#[derive(Clone, Debug)]
pub struct NodeData {
	pub nodes: Vec<u16>,
	pub param1: Vec<u8>,
	pub param2: Vec<u8>
}

impl NodeData {
	pub fn deserialize<T: Read>(src: &mut T) -> Result<Self, MapBlockError> {
		let mut node_bytes = vec_with_len(NODE_COUNT * 2);
		src.read_exact(&mut node_bytes)?;
		let mut nodes = vec_with_len(NODE_COUNT);
		BigEndian::read_u16_into(&node_bytes, &mut nodes);

		let mut param1 = vec_with_len(NODE_COUNT);
		src.read_exact(&mut param1)?;

		let mut param2 = vec_with_len(NODE_COUNT);
		src.read_exact(&mut param2)?;

		Ok(Self {
			nodes,
			param1,
			param2
		})
	}

	pub fn decompress(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError> {
		let start = src.position();
		let mut decoder = ZlibDecoder::new(src);

		let node_data = Self::deserialize(&mut decoder)?;

		// Fail if there is leftover compressed data.
		if decoder.read(&mut [0])? > 0 {
			return Err(MapBlockError::BadData);
		}

		let total_in = decoder.total_in();
		let src = decoder.into_inner();
		src.set_position(start + total_in);

		Ok(node_data)
	}

	pub fn serialize<T: Write>(&self, dst: &mut T) {
		// This allocation seems slow, but writing u16s iteratively is slower.
		let mut node_bytes = vec_with_len(NODE_COUNT * 2);
		BigEndian::write_u16_into(&self.nodes,
			&mut node_bytes[..NODE_COUNT * 2]);

		dst.write_all(&node_bytes).unwrap();
		dst.write_all(&self.param1).unwrap();
		dst.write_all(&self.param2).unwrap();
	}

	pub fn compress<T: Write>(&self, dst: &mut T) {
		let mut encoder = ZlibEncoder::new(dst, Compression::default());
		self.serialize(&mut encoder);
		encoder.finish().unwrap();
	}
}
