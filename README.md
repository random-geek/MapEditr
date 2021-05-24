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

## Compilation/Installation

### Option 1: Pre-built releases

If you are using Windows and don't have Rust installed, you can download a
build of the latest release of MapEditr from the [Releases page][3]. Only
64-bit Windows builds are currently available.

To run the `mapeditr` command from anywhere, the path to the executable file
must be included in your system's Path variable. [Here is one article][4]
explaining how to edit the Path variable on Windows.

### Option 2: Install using Cargo

This method works on any operating system. To use Cargo, you must have Rust
installed first, which can be downloaded from [the Rust website][5]. Then,
simply run:

`cargo install --git https://github.com/random-geek/MapEditr.git`

This will download MapEditr and install it to `$HOME/.cargo/bin`. After
installing, you should be able to run MapEditr from anywhere with the
`mapeditr` command.

### Option 3: Build normally

If you don't wish to install MapEditr, you can build it normally using Cargo.
In the MapEditr directory, run:

`cargo build --release`

The `--release` flag is important, as it produces an optimized executable which
runs much faster than the default, unoptimized version. The compiled executable
will be in the `target/release` directory.

## Usage

For an overview of how MapEditr works and a listing of commands and their
usages, see [Manual.md](Manual.md).

These are just a few of the useful things you can do with MapEditr:

- Remove unknown nodes left by old mods with `replacenodes`.
- Build extremely long walls and roads in seconds using `fill`.
- Selectively delete entities and/or dropped items using `deleteobjects`.
- Combine multiple worlds or map saves with `overlay`.

## License

MapEditr is under the terms of the MIT license as defined in `LICENSE.txt`.

Additionally, if you use code from MapEditr in another project, I would
greatly appreciate a reasonable acknowledgement/attribution of MapEditr in your
project's readme or documentation.

## Acknowledgments

The [Minetest][6] project has been rather important for the making of
MapEdit/MapEditr, for obvious reasons.

Some parts of the original MapEdit code were adapted from AndrejIT's
[map_unexplore][7] project. All due credit goes to the author(s) of that
project.

Thank you also to ExeterDad and the moderators of the late Hometown server, for
partially inspiring MapEdit/MapEditr.

[1]: https://github.com/Uberi/Minetest-WorldEdit
[2]: https://github.com/random-geek/MapEdit
[3]: https://github.com/random-geek/MapEditr/releases
[4]: https://www.howtogeek.com/118594/how-to-edit-your-system-path-for-easy-command-line-access/
[5]: https://www.rust-lang.org
[6]: https://github.com/minetest/minetest
[7]: https://github.com/AndrejIT/map_unexplore
