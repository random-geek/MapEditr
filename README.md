# MapEditr

MapEditr is a command-line tool for fast manipulation of Minetest worlds. It
can replace nodes and items, fill areas, combine parts of different worlds, and
much more.

This tool is functionally similar to [WorldEdit][1], but designed for large
operations that would be impractical to do within Minetest. Since it is mainly
optimized for speed, MapEditr lacks some of the more specialty features of
WorldEdit.

MapEditr was originally based on [MapEdit][2], except written in Rust rather
than Python (hence the added "r"). Switching to a compiled language will make
MapEditr more robust and easier to maintain in the future.

[1]: https://github.com/Uberi/Minetest-WorldEdit
[2]: https://github.com/random-geek/MapEdit

## Compilation/Installation

TODO: Pre-built binaries

To compile from source, you must have Rust installed first, which can be
downloaded from [the Rust website][3]. Then, in the MapEditr directory, simply
run:

`cargo build --release`

The `--release` flag is important, as it produces an optimized executable which
runs much faster than the default, unoptimized version.

[3]: https://www.rust-lang.org

## Usage

For an overview of how MapEditr works and a listing of commands and their
usages, see [Manual.md](Manual.md).

Some useful things you can do with MapEditr:

- Remove unknown nodes left by old mods with `replacenodes`.
- Build extremely long walls and roads in seconds using `fill`.
- Combine multiple worlds or map saves with `overlay`.

## License

TODO

## Acknowledgments

The [Minetest][4] project has been rather important for the making of
MapEdit/MapEditr, for obvious reasons.

Some parts of the original MapEdit code were adapted from AndrejIT's
[map_unexplore][5] project. All due credit goes to the author(s) of that
project.

Thank you also to ExeterDad and the moderators of the late Hometown server, for
partially inspiring MapEdit/MapEditr.

[4]: https://github.com/minetest/minetest
[5]: https://github.com/AndrejIT/map_unexplore
