use super::{Command, ArgResult};

use crate::unwrap_or;
use crate::spatial::Vec3;
use crate::instance::{ArgType, InstArgs, InstBundle};
use crate::map_block::{MapBlock, NodeMetadataList, NodeMetadataListExt};
use crate::utils::{query_keys, to_bytes, fmt_big_num};

const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';


fn do_replace(inv: &mut Vec<u8>, item: &[u8], new_item: &[u8], del_meta: bool)
	-> u64
{
	let delete = new_item.is_empty();
	let mut new_inv = Vec::with_capacity(inv.len());
	let mut mods = 0;

	for line in inv.split(|&x| x == NEWLINE) {
		if line.is_empty() {
			// Necessary because of newline after final EndInventory
			continue;
		}
		// Max 5 parts: Item <name> <count> <wear> <metadata>
		let mut parts = line.splitn(5, |&x| x == SPACE);
		if parts.next() == Some(b"Item") && parts.next() == Some(item) {
			if delete {
				new_inv.extend_from_slice(b"Empty");
			} else {
				new_inv.extend_from_slice(b"Item ");
				new_inv.extend_from_slice(new_item);

				if del_meta { // Only re-serialize necessary parts
					let count = parts.next().unwrap_or(b"1");
					let wear = parts.next().unwrap_or(b"0");
					if count != b"1" || wear != b"0" {
						new_inv.push(SPACE);
						new_inv.extend_from_slice(count);
					}
					if wear != b"0" {
						new_inv.push(SPACE);
						new_inv.extend_from_slice(wear);
					}
				} else {
					for part in parts {
						new_inv.push(SPACE);
						new_inv.extend_from_slice(part);
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
	let item = to_bytes(inst.args.item.as_ref().unwrap());
	let new_item = inst.args.new_item.as_ref().map(to_bytes)
		.unwrap_or(if inst.args.delete_item { vec![] } else { item.clone() });

	let nodes: Vec<_> = inst.args.nodes.iter().map(to_bytes).collect();
	let keys = query_keys(&mut inst.db, &mut inst.status,
		&nodes, inst.args.area, inst.args.invert, true);

	inst.status.begin_editing();
	let mut item_mods: u64 = 0;
	let mut node_mods: u64 = 0;

	for key in keys {
		inst.status.inc_done();
		let data = inst.db.get_block(key).unwrap();
		let mut block = unwrap_or!(MapBlock::deserialize(&data),
			{ inst.status.inc_failed(); continue; });

		let node_data = block.node_data.get_ref();
		let node_ids: Vec<_> = nodes.iter()
			.filter_map(|n| block.nimap.get_id(n)).collect();
		if !nodes.is_empty() && node_ids.is_empty() {
			continue; // Block doesn't contain any of the required nodes.
		}

		let mut meta = unwrap_or!(
			NodeMetadataList::deserialize(block.metadata.get_ref()),
			{ inst.status.inc_failed(); continue; });

		let block_corner = Vec3::from_block_key(key) * 16;
		let mut modified = false;

		for (&idx, data) in &mut meta {
			let pos = Vec3::from_u16_key(idx);
			let abs_pos = pos + block_corner;
			if let Some(a) = inst.args.area {
				if a.contains(abs_pos) == inst.args.invert {
					continue;
				}
			}
			if !node_ids.is_empty()
				&& !node_ids.contains(&node_data.nodes[idx as usize])
			{
				continue;
			}

			let i_mods = do_replace(&mut data.inv, &item, &new_item,
				inst.args.delete_meta);
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


fn verify_args(args: &InstArgs) -> ArgResult {
	if args.new_item.is_none() && !args.delete_item && !args.delete_meta {
		return ArgResult::error(
			"new_item is required unless --delete or --deletemeta is used.");
	} else if args.new_item.is_some() && args.delete_item {
		return ArgResult::error(
			"Cannot delete items if new_item is specified.");
	}
	ArgResult::Ok
}


pub fn get_command() -> Command {
	Command {
		func: replace_in_inv,
		verify_args: Some(verify_args),
		args: vec![
			(ArgType::Item, "Name of the item to replace/delete"),
			(ArgType::NewItem, "Name of the new item, if replacing items."),
			(ArgType::DeleteMeta, "Delete metadata of affected items."),
			(ArgType::DeleteItem, "Delete items instead of replacing them."),
			(ArgType::Area(false), "Area in which to modify inventories"),
			(ArgType::Invert, "Modify inventories outside the given area."),
			(ArgType::Nodes, "Names of nodes to modify inventories of"),
		],
		help: "Replace or delete items in node inventories."
	}
}


#[cfg(test)]
mod tests {
	use super::do_replace;

	#[test]
	fn test_replace_in_inv() {
		let original = b"\
			List main 10\n\
			Width 5\n\
			Item tools:pickaxe 1 300\n\
			Item test:foo\n\
			Item test:foo 3\n\
			Item test:foo 10 32768\n\
			Empty\n\
			Item test:foo 1 0 \x01some variable\x02some value\x03\n\
			Item test:foo 1 1234 \x01color\x02#FF00FF\x03\n\
			Item test:bar 20\n\
			Item test:foo 42 0 \x01random_number\x02892\x03\n\
			Item test:foo 99 100 \x01description\x02test metadata\x03\n\
			EndInventoryList\n\
			EndInventory\n";
		let replace = b"\
			List main 10\n\
			Width 5\n\
			Item tools:pickaxe 1 300\n\
			Item test:bar\n\
			Item test:bar 3\n\
			Item test:bar 10 32768\n\
			Empty\n\
			Item test:bar 1 0 \x01some variable\x02some value\x03\n\
			Item test:bar 1 1234 \x01color\x02#FF00FF\x03\n\
			Item test:bar 20\n\
			Item test:bar 42 0 \x01random_number\x02892\x03\n\
			Item test:bar 99 100 \x01description\x02test metadata\x03\n\
			EndInventoryList\n\
			EndInventory\n";
		let delete = b"\
			List main 10\n\
			Width 5\n\
			Item tools:pickaxe 1 300\n\
			Empty\n\
			Empty\n\
			Empty\n\
			Empty\n\
			Empty\n\
			Empty\n\
			Item test:bar 20\n\
			Empty\n\
			Empty\n\
			EndInventoryList\n\
			EndInventory\n";
		let replace_delete_meta = b"\
			List main 10\n\
			Width 5\n\
			Item tools:pickaxe 1 300\n\
			Item test:bar\n\
			Item test:bar 3\n\
			Item test:bar 10 32768\n\
			Empty\n\
			Item test:bar\n\
			Item test:bar 1 1234\n\
			Item test:bar 20\n\
			Item test:bar 42\n\
			Item test:bar 99 100\n\
			EndInventoryList\n\
			EndInventory\n";

		let mut inv = original.to_vec();
		do_replace(&mut inv, b"test:foo", b"test:bar", false);
		assert_eq!(&inv, replace);

		let mut inv = original.to_vec();
		do_replace(&mut inv, b"test:foo", b"", false);
		assert_eq!(&inv, delete);

		let mut inv = original.to_vec();
		do_replace(&mut inv, b"test:foo", b"test:bar", true);
		assert_eq!(&inv, replace_delete_meta);
	}
}
