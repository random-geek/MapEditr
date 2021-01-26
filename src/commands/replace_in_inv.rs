use super::Command;

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList};
use crate::utils::{query_keys, fmt_big_num};

const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';


fn do_replace(inv: &mut Vec<u8>, item: &[u8], new_item: &[u8], del_meta: bool)
	-> u64
{
	let delete = new_item == b"Empty";
	let mut new_inv = Vec::with_capacity(inv.len());
	let mut mods = 0;

	for line in inv.split(|&x| x == NEWLINE) {
		let mut parts = line.splitn(4, |&x| x == SPACE);

		if parts.next() == Some(b"Item") && parts.next() == Some(item) {
			if delete {
				new_inv.extend_from_slice(b"Empty");
			} else {
				new_inv.extend_from_slice(b"Item ");
				new_inv.extend_from_slice(new_item);

				if let Some(count) = parts.next() {
					new_inv.push(SPACE);
					new_inv.extend_from_slice(count);
				}
				if !del_meta {
					if let Some(meta) = parts.next() {
						new_inv.push(SPACE);
						new_inv.extend_from_slice(meta);
					}
				}
			}
			mods += 1;
		} else {
			new_inv.extend_from_slice(line);
		}
		new_inv.push(NEWLINE);
	}

	if mods > 0 {
		*inv = new_inv;
	}
	mods
}


fn replace_in_inv(inst: &mut InstBundle) {
	let item = inst.args.item.as_ref().unwrap().as_bytes().to_owned();
	let new_item = inst.args.new_item.as_ref().unwrap().as_bytes().to_owned();
	let del_meta = false;
	let node = inst.args.node.as_ref().map(|s| s.as_bytes().to_owned());

	let keys = query_keys(&mut inst.db, &mut inst.status,
		node.iter().collect(), inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut item_mods: u64 = 0;
	let mut node_mods: u64 = 0;

	for key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data), continue);

		let node_data = block.node_data.get_ref();
		let node_id = node.as_deref().and_then(|n| block.nimap.get_id(n));
		if node.is_some() && node_id.is_none() {
			continue; // Block doesn't contain the required node.
		}

		let mut meta = unwrap_or!(
			NodeMetadataList::deserialize(block.metadata.get_ref()), continue);

		let block_corner = Vec3::from_block_key(key) * 16;
		let mut modified = false;

		for (&idx, data) in &mut meta.list {
			let pos = Vec3::from_u16_key(idx);
			let abs_pos = pos + block_corner;
			if let Some(a) = inst.args.area {
				if a.contains(abs_pos) == inst.args.invert {
					continue;
				}
			}
			if let Some(id) = node_id {
				if node_data.nodes[idx as usize] != id {
					continue;
				}
			}

			let i_mods = do_replace(&mut data.inv, &item, &new_item, del_meta);
			item_mods += i_mods;
			if i_mods > 0 {
				node_mods += 1;
				modified = true;
			}
		}

		if modified {
			*block.metadata.get_mut() = meta.serialize(block.version);
			inst.db.set_block(key, &block.serialize()).unwrap();
		}
	}

	inst.status.end_editing();
	inst.status.log_info(format!("Replaced {} itemstacks in {} nodes.",
		fmt_big_num(item_mods), fmt_big_num(node_mods)));
}


pub fn get_command() -> Command {
	Command {
		func: replace_in_inv,
		verify_args: None,
		args: vec![
			(ArgType::Item, "Name of item to replace"),
			(ArgType::NewItem, "Name of new item to replace with"),
			(ArgType::Area(false), "Area in which to modify inventories"),
			(ArgType::Invert, "Modify inventories outside the given area."),
			(ArgType::Node(false), "Node to modify inventories of")
		],
		help: "Replace items in node inventories."
	}
}
