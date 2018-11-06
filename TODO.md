# TODO

## Features
 * Add verbosity levels, with -v, and maybe even -vv (let's not get crazy though)
 * Add more flags and LS\_COLORS to the ls subcommand
 * Add a disassembler
   * Look into http://www.keystone-engine.org/
   * Don't print to stdout, but rather a file
   * Add the ability to pick which segment(s) are disassembled
   * Sometimes I just want to disassemble a single instruction, add that.
 * Make the info command give more useful info (plus add verbosity levels with -v, -vv, and so on...)
 * Use std::path::MAIN\_SEPARATOR for Windows support
 * Add a thing like print\_layout for the dol, that prints brief information about all the segments (support -v for added verbosity)
   * Where would this be used? Info?
 * Patch file generator for mods???
 * Add cool stuff for getting info on the rom like a tree command to view the file system as a tree
 * Add an option to rebuild that doesn't rebuild the &&systemdata directory (should that be the default? probably?)
   * This is in the code, there's just no command line option for it
 * Write files toward the end of the iso to improve speed?
   * Wait, does anything need to be aligned more than 2 bytes? (http://www.gc-forever.com/forums/viewtopic.php?p=1487&sid=a5f89e4c4ee820c1305b27babf50eccd#p1487)
 * Add a way to specify a certain alignment for files matching a regex?
 * Add more subcommands
 * Add a progress indicator for the rebuild command
 * Add options or commands specifically for apploaders, dol files, etc... so the whole iso isn't needed
 * Add a GCRebuilder compatibility mode (this'd just add weird extra zeros to some files, but that may not even be necessary idk)
 * Add offsets for other commands (info has it now)
   * This could be used to support just passing a certain file to a subcommand that only needs that part of the rom, like a dol or fst
 * Make sure it works for Japanese ROMs and unicode

## Usage improvements/UX
 * Make the section names in `gcmod extract -s` more intuitive
 * Add a proper way to install it (then add that to the readme)
 * Get rid of the `-V/--version` flag from the subcommand's help option, make sure it's only in the main help
 * Improve error messages
   * Add errors for file permission errors
 * More error handling, especially for corrupt isos

## Refactoring
 * Make a version of DirectoryEntry::iter\_contents that is recursive/goes through sub dirs
 * Only make stuff public if it needs to/should be be
 * Rename LayoutSection to Section???
 * Should ROMLayout have a BinaryHeap instead of a Vec?
 * Fix the style to be consistent with the official Rust style guide
 * Get rid of that one unsafe block
 * LayoutSection needs to be split into separate traits, like RebuildableSection, ExtractableSection, Section, etc...
 * Add an offset param to extract\_section (still?)
 * Make Entry easier to use
 * Make a DOL struct, which contains a DOLHeader struct, and would probably contain the segments directly
 * Add an extract associated function to layout\_section, rename the extract functions to be more meaningful
 * Add more useful, generic stuff to extract\_section, like an error message if the file already exists
 * Move stuff like consts and functions from src/lib.rs into something like src/utility.rs
 * Refactor that gross loop in FST::new
   * Make it easy to understand what's going on
 * Remove the offset param in Game::new
 * Make methods or functions for adding sections or files to a btree map, maybe make a struct to generalize the thing going on with make\_sections\_btree
   * Is this still relevant?
 * Cow could probably be used a few more places to reduce allocations, but the potential benefits probably aren't worth the trouble. Look into this though
   * Wait a minute... Why don't I just add a name property or something to Segment?
 * Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
 * Anything in the source with a `TODO: ` label
 * Should FST be renamed to FileSystemTable?
 * Stop using `file` to refer to entries in general. `file` in a name should always imply it's just a file, not a directory. `entry` needs to be the generic term for something that may be a file or directory.
 * Make a type alias for Path to differentiate between paths on the rom and on the computer?

## Bugs
 * The reported "files extracted/added" values are one to high because the root entry shouldn't really count

## Misc
 * Explain Rust style guidelines (basically it's the official guidelines except for matches)
 * Keep working on the documentation on Google Docs
 * Make an API (it'd be a separate repo of course)
 * Add tests
 * Add a description for the project on GitHub

This is golden: http://www.gc-forever.com/forums/viewtopic.php?p=1487&sid=a5f89e4c4ee820c1305b27babf50eccd#p1487
