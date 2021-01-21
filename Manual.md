# The MapEditr Manual

## Introduction

MapEditr is a command-line tool for editing Minetest worlds, specifically
Minetest maps. Note that MapEditr is not a mod or plugin; it is a separate
program which operates independently of Minetest.

Minetest *worlds* are stored in the `worlds` folder within Minetest's
installation directory. Each world is a folder containing a *map database*,
usually named `map.sqlite`, among other files. The map database contains the
physical layout of that world, including all nodes (blocks) and objects (mobs,
etc.). This file is what MapEditr reads and edits.

Minetest stores map data in *mapblocks*. A single map block is a cubical,
16x16x16 node area of the map. The lower southwestern corner of a mapblock
(towards -X, -Y, -Z) is always at coordinates divisible by 16, e.g.
(0, 16, -48) or the like.

For most commands to work, the mapblocks to be read and modified must already
be generated within Minetest. This can be achieved by either exploring the area
in-game, or by using Minetest's built-in `/emergeblocks` command.

MapEditr supports map format versions 25 through 28, meaning all worlds
created since Minetest version 0.4.2-rc1 (released July 2012) should be
supported. Unsupported mapblocks will be skipped (TODO).

## General usage

`mapedit [-h] <map> <subcommand>`

Arguments:

- `-h`: Show a help message and exit.
- `<map>`: Path to the Minetest world to edit; this can be either a world
directory or a `map.sqlite` file. Note that only worlds with SQLite map
databases are currently supported. This file will be modified, so *always* shut
down the game/server before executing the command.
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

### deleteobjects

Usage: `deleteobjects [--obj <obj>] [--items] [--p1 x y z] [--p2 x y z] [--invert]`

Delete objects (entities) of a certain name and/or within a certain area.

Arguments:

- `--obj`: Name of object to search for, e.g. "boats:boat". If not specified,
all objects will be deleted.
- `--items`: Search for only item entities (dropped items). If this flag is
set, `--obj` can optionally be used to specify an item name.
- `--p1, --p2`: Area in which to delete objects. If not specified, objects will
be deleted across the entire map.
- `--invert`: Delete objects *outside* the given area.

### fill

Usage: `fill --p1 x y z --p2 x y z [--invert] <new_node>`

Fills the given area with one node. The affected mapblocks must be already
generated for fill to work.

This command does not affect param2, node metadata, etc.

Arguments:

- `new_node`: Name of node to fill the area with.
- `--p1, --p2`: Area to fill.
- `--invert`: Fill everything *outside* the given area.

### clone

Usage: `clone --p1 x y z --p2 x y z --offset x y z`

Clone (copy) the given area to a new location.

Arguments:

- `--p1, --p2`: Area to copy from.
- `--offset`: Offset to shift the area by. For example, to copy an area 50
nodes upward (positive Y direction), use `--offset 0 50 0`.

This command copies nodes, param1, param2, and metadata. Nothing will be copied
into mapblocks that are not yet generated.

### overlay

Usage: `overlay [--p1 x y z] [--p2 x y z] [--invert] [--offset x y z] <input_map>`

Copy part or all of an input map into the main map.

Arguments:

- `input_map`: Path to input map file. This will not be modified.
- `--p1, --p2`: Area to copy from. If not specified, MapEditr will try to
copy everything from the input map file.
- `--invert`: If present, copy everything *outside* the given area.
- `--offset`: Offset to move nodes by when copying; default is no offset.
Currently, an offset cannot be used with an inverted selection.

This command will always copy nodes, param1 and param2, and metadata. If no
offset is used, entities and node timers may also be copied.

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

### setparam2

Usage: `setparam2 [--node <node>] [--p1 x y z] [--p2 x y z] [--invert] <param2_val>`

Set param2 values of a certain node and/or within a certain area.

Arguments:

- `param2_val`: Param2 value to set, between 0 and 255.
- `--node`: Name of node to modify. If not specified, the param2 values of
all nodes will be set.
- `--p1, --p2`: Area in which to set param2. Required if `--node` is
not specified.
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






# Danger Zone!

### `deletemeta`

**Usage:** `deletemeta [--searchnode <searchnode>] [--p1 x y z] [--p2 x y z] [--invert]`

Delete metadata of a certain node and/or within a certain area. This includes node inventories as well.

Arguments:

- **`--searchnode`**: Name of node to search for. If not specified, the metadata of all nodes will be deleted.
- **`--p1, --p2`**: Area in which to delete metadata. Required if `searchnode` is not specified.
- **`--invert`**: Only delete metadata *outside* the given area.

### `setmetavar`

**Usage:** `setmetavar [--searchnode <searchnode>] [--p1 x y z] [--p2 x y z] [--invert] <metakey> <metavalue>`

Set a variable in node metadata. This only works on metadata where the variable is already set.

Arguments:

- **`metakey`**: Name of variable to set, e.g. `infotext`, `formspec`, etc.
- **`metavalue`**: Value to set variable to. This should be a string.
- **`--searchnode`**: Name of node to search for. If not specified, the variable will be set for all nodes that have it.
- **`--p1, --p2`**: Area in which to search. Required if `searchnode` is not specified.
- **`--invert`**: Only search for nodes *outside* the given area.

### `replaceininv`

**Usage:** ` replaceininv [--deletemeta] [--searchnode <searchnode>] [--p1 x y z] [--p2 x y z] [--invert] <searchitem> <replaceitem>`

Replace a certain item with another in node inventories.
To delete items instead of replacing them, use "Empty" (with a capital E) for `replacename`.

Arguments:

- **`searchitem`**: Item to search for in node inventories.
- **`replaceitem`**: Item to replace with in node inventories.
- **`--deletemeta`**: Delete metadata of replaced items. If not specified, any item metadata will remain unchanged.
- **`--searchnode`**: Name of node to to replace in. If not specified, the item will be replaced in all node inventories.
- **`--p1, --p2`**: Area in which to search for nodes. If not specified, items will be replaced across the entire map.
- **`--invert`**: Only search for nodes *outside* the given area.

**Tip:** To only delete metadata without replacing the nodes, use the `--deletemeta` flag, and make `replaceitem` the same as `searchitem`.

### `deletetimers`

**Usage:** `deletetimers [--searchnode <searchnode>] [--p1 x y z] [--p2 x y z] [--invert]`

Delete node timers of a certain node and/or within a certain area.

Arguments:

- **`--searchnode`**: Name of node to search for. If not specified, the node timers of all nodes will be deleted.
- **`--p1, --p2`**: Area in which to delete node timers. Required if `searchnode` is not specified.
- **`--invert`**: Only delete node timers *outside* the given area.
