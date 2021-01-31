# The MapEditr Manual

## Introduction

MapEditr is a command-line tool for editing Minetest worlds. Note that MapEditr
is not a mod or plugin; it is a separate program which operates independently
of Minetest.

MapEditr reads and edits *map databases*, usually a file named `map.sqlite`
within each Minetest world directory. As such, the terms "world" and "map" may
be used interchangeably.

For most commands to work, the areas of the map to be read/modified must
already be generated. This can be done by either exploring the area in-game,
or by using Minetest's built-in `/emergeblocks` command.

MapEditr supports all maps created since Minetest version 0.4.2-rc1, released
July 2012. Any unsupported areas of the map will be skipped (TODO). Note that
only SQLite format maps are currently supported.

## General usage

`mapeditr [-h] <map> <subcommand>`

Arguments:

- `-h`: Show a help message and exit.
- `<map>`: Path to the Minetest world/map to edit; this can be either a world
directory or a `map.sqlite` file within a world folder. This file will be
modified, so *always* shut down the game/server before executing any command.
- `<subcommand>`: Command to execute. See "Commands" section below.

### Common command arguments

- `--p1 <x> <y> <z>` and `--p2 <x> <y> <z>`: Used to select a box-shaped
area with corners at `p1` and `p2`, similarly to how WorldEdit's area selection
works. Any two opposite corners can be used. These coordinates can be found
using Minetest's F5 debug menu.
- Node/item names: includes `node`, `new_node`, etc. Must be the full name,
e.g. "default:stone", not just "stone".

### Other tips

Text-like arguments can be surrounded with "quotes" if they contain spaces.

Due to technical limitations, MapEditr will often leave lighting glitches. To
fix these, use Minetest's built-in `/fixlight` command, or the equivalent
WorldEdit `//fixlight` command.

## Commands

### clone

Usage: `clone --p1 x y z --p2 x y z --offset x y z`

Clone (copy) the given area to a new location.

Arguments:

- `--p1, --p2`: Area to copy from.
- `--offset`: Offset to shift the area by. For example, to copy an area 50
nodes upward (positive Y direction), use `--offset 0 50 0`.

This command copies nodes, param1, param2, and metadata. Nothing will be copied
into mapblocks that are not yet generated.

### deleteblocks

Usage: `deleteblocks --p1 x y z --p2 x y z [--invert]`

Deletes all mapblocks in the given area.

Arguments:

- `--p1, --p2`: Area to delete from. Only mapblocks fully inside this area
will be deleted.
- `--invert`: Delete only mapblocks that are fully *outside* the given
area.

**Note:** Deleting mapblocks is *not* the same as filling them with air! Mapgen
will be invoked where the blocks were deleted, and this sometimes causes
terrain glitches.

### deletemeta

Usage: `deletemeta [--node <node>] [--p1 x y z] [--p2 x y z] [--invert]`

Delete metadata of a certain node and/or within a certain area. This includes
node inventories as well.

Arguments:

- `--node`: Name of node to modify. If not specified, the metadata of all
nodes will be deleted.
- `--p1, --p2`: Area in which to delete metadata. If not specified, metadata
will be deleted everywhere.
- `--invert`: Only delete metadata *outside* the given area.

### deleteobjects

Usage: `deleteobjects [--obj <object>] [--items [item]] [--p1 x y z] [--p2 x y z] [--invert]`

Delete objects/entities, including item entities (dropped items).

Arguments:

- `--obj`: Name of object to delete, e.g. "boats:boat". If not specified,
all objects will be deleted.
- `--items [item]`: Delete item entities (dropped items). If an optional item
name is specified, only items with that name will be deleted.
- `--p1, --p2`: Area in which to delete objects. If not specified, objects will
be deleted everywhere.
- `--invert`: Delete objects *outside* the given area.

### deletetimers

Usage: `deletetimers [--node <node>] [--p1 x y z] [--p2 x y z] [--invert]`

Delete node timers of a certain node and/or within a certain area.

Arguments:

- `--node`: Name of node to modify. If not specified, the node timers of all
nodes will be deleted.
- `--p1, --p2`: Area in which to delete node timers.
- `--invert`: Only delete node timers *outside* the given area.

### fill

Usage: `fill --p1 x y z --p2 x y z [--invert] <new_node>`

Fills the given area with one node. The affected mapblocks must be already
generated for fill to work.

This command does not affect param2, node metadata, etc.

Arguments:

- `new_node`: Name of node to fill the area with.
- `--p1, --p2`: Area to fill.
- `--invert`: Fill everything *outside* the given area.

### overlay

Usage: `overlay <input_map> [--p1 x y z] [--p2 x y z] [--invert] [--offset x y z]`

Copy part or all of a source map into the main map.

Arguments:

- `input_map`: Path to source map/world. This will not be modified.
- `--p1, --p2`: Area to copy from. If not specified, MapEditr will try to
copy everything from the input map file.
- `--invert`: If present, copy everything *outside* the given area.
- `--offset`: Offset to move nodes by when copying; default is no offset.
Currently, an offset cannot be used with an inverted selection.

This command will always copy nodes, param1, param2, and metadata. If no
offset is used, objects/entities and node timers may also be copied.

To ensure that all data is copied, make sure the edges of the selection are
generated in the destination map, or the entire selection if an offset is used.

**Tip:** Overlay will be significantly faster if no offset is used, as
mapblocks can be copied verbatim.

### replaceininv

Usage: `replaceininv [--delete] [--deletemeta] [--nodes <nodes>] [--p1 x y z]
[--p2 x y z] [--invert] <item> [new_item]`

Replace or delete certain items in node inventories.

Arguments:

- `item`: Name of item to replace/delete
- `new_item`: Name of new item, if replacing items.
- `--delete`: Delete items instead of replacing them.
- `--deletemeta`: Delete metadata of items. May be used with or without
`new_item`, depending on whether items should also be replaced.
- `--nodes`: Names of one or more nodes to replace in. If not specified, the
item will be replaced in all node inventories.
- `--p1, --p2`: Area in which to modify node inventories. If not specified,
items will be replaced in all node inventories.
- `--invert`: Only modify node inventories *outside* the given area.

Examples:

Replace all written books in chests with unwritten books, deleting metadata:

`replaceininv default:book_written default:book --deletemeta --nodes
default:chest default:chest_locked`

### replacenodes

Usage: `replacenodes [--p1 x y z] [--p2 x y z] [--invert] <node> <new_node>`

Replace all of one node with another node. Can be used to remove unknown nodes
or swap a node that changed names.

This command does not affect param2, metadata, etc.

Arguments:

- `node`: Name of node to replace.
- `new_node`: Name of node to replace with.
- `--p1, --p2`: Area in which to replace nodes. If not specified, nodes
will be replaced across the entire map.
- `--invert`: Only replace nodes *outside* the given area.

### setmetavar

Usage: `setmetavar [--node <node>] [--p1 x y z] [--p2 x y z] [--invert] <key> <value>`

Set a variable in node metadata. This only works on metadata where the variable
is already set.

Arguments:

- `key`: Name of variable to set, e.g. `infotext`, `formspec`, etc.
- `value`: Value to set variable to. This should be a string.
- `--node`: Name of node to modify. If not specified, the variable will be
set for all nodes that have it.
- `--p1, --p2`: Area in which to modify nodes.
- `--invert`: Only modify nodes *outside* the given area.

### setparam2

Usage: `setparam2 [--node <node>] [--p1 x y z] [--p2 x y z] [--invert] <param2_val>`

Set param2 values of a certain node and/or within a certain area.

Arguments:

- `param2_val`: Param2 value to set, between 0 and 255.
- `--node`: Name of node to modify. If not specified, the param2 values of
all nodes will be set.
- `--p1, --p2`: Area in which to set param2.
- `--invert`: Only set param2 *outside* the given area.

### vacuum

Usage: `vacuum`

Vacuums the database. This reduces the size of the database, but may take a
long time.

All this does is perform an SQLite `VACUUM` command. This shrinks and optimizes
the database by efficiently "repacking" all mapblocks. No map data is changed
or deleted.

**Note:** Because data is copied into another file, vacuum could require
as much free disk space as is already occupied by the map. For example, if
map.sqlite is 10 GB, make sure you have **at least 10 GB** of free space!
