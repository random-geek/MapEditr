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

impl Compress for NodeData {
	fn compress(&self, dst: &mut Cursor<Vec<u8>>) {
		let mut encoder = ZlibEncoder::new(dst, Compression::default());

		let mut node_bytes = vec_with_len(NODE_COUNT * 2);
		BigEndian::write_u16_into(&self.nodes,
			&mut node_bytes[..NODE_COUNT * 2]);

		encoder.write_all(&node_bytes).unwrap();
		encoder.write_all(&self.param1).unwrap();
		encoder.write_all(&self.param2).unwrap();
		encoder.finish().unwrap();
	}

	fn decompress(src: &mut Cursor<&[u8]>) -> Result<Self, MapBlockError> {
		let start = src.position();
		let mut decoder = ZlibDecoder::new(src);

		let mut node_bytes = vec_with_len(NODE_COUNT * 2);
		decoder.read_exact(&mut node_bytes)?;
		let mut nodes = vec_with_len(NODE_COUNT);
		BigEndian::read_u16_into(&node_bytes, &mut nodes);

		let mut param1 = vec_with_len(NODE_COUNT);
		decoder.read_exact(&mut param1)?;

		let mut param2 = Vec::with_capacity(NODE_COUNT);
		decoder.read_to_end(&mut param2)?;
		if param2.len() != NODE_COUNT {
			return Err(MapBlockError::DataError)
		}

		let total_in = decoder.total_in();
		let src = decoder.into_inner();
		src.set_position(start + total_in);

		Ok(Self {
			nodes,
			param1,
			param2
		})
	}
}
