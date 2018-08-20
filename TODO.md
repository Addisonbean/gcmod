# TODO

## Next

* Only make stuff public if it needs to/should be be
* Make the disassembler more practical/easier to use
	* Don't print to stdout, but rather a file
	* Add the ability to pick which segment(s) are disassembled
	* Sometimes I just want to disassemble a single instruction, add that.
* Get rid of the useless Game::extract_\... methods
* Rename LayoutSection to Section???
* Use Vec::binary\_search in ROMLayout::find\_offset, find a way to make sure that vec stays sorted (probably by not making it a Vec)
* Make the section names in `gcmod extract -s` more intuitive
* Fix the style to be consistent with the official Rust style guide
	* Find a better way to use clap, try the macros
* Add a way to install it (then add that to the readme)
* Explain Rust style guidelines (basically it's the official guidelines except for matches and explain how to break function signatures)
* Make the info command give more useful info (plus add verbosity levels with -v, -vv, and so on...)
* LayoutSection needs to be split into separate traits, like RebuildableSection, ExtractableSection, Section, etc...
* Add an offset param to extract\_section
* Add more methods to layout\_section, like rebuild and extract related stuff?
* Make the info subcommand always give the same output for a section, regardless of whether -t or -o is used.
	* Add verbosity levels, with -v, and maybe even -vv (let's not get crazy though)
* Use std::path::MAIN\_SEPARATOR for Windows support
* Refactor everything in main.rs
* FST.entry\_with\_name seems too complicated, using entries needs to be easier
* Make a DOL struct, which contains a DOLHeader struct, and would probably contain the segments directly
* Add an extract associated function to layout\_section, rename the extract functions to be more meaningful
* Add more useful, generic stuff to extract\_section, like an error message if the file already exists
* Move stuff like consts and functions from src/lib.rs into something like src/utility.rs
* Keep working on the documentation on Google Docs
* Set shell exit code/status after failure

* Warn if the iso to be rebuilt already exists
* Refactor the rebuilding process, maybe create a ROMRebuilder struct...
* Get rid of the `-V/--version` flag from the subcommand's help option, make sure it's only in the main help
* Add a thing like print\_layout for the dol, that prints brief information about all the segments (support -v for added verbosity)
* Patch file generator for mods???
* Remove the offset param in Game::new
* Add cool stuff for getting info on the rom like a tree command to view the file system as a tree, an ls or list-files subcommand, cool stuff like that
* Improve the modules exported by lib.rs, like no more gamecube\_iso\_assistant::apploader::Apploader
* Make methods or functions for adding sections or files to a btree map, maybe make a struct to generalize the thing going on with make\_sections\_btree
* Make a function like Game::is\_valid\_rom that checks to see if it has the magic byte and if it's the right size?
* Add an option to rebuild that doesn't rebuild the &&systemdata directory (should that be the default? probably?)
* Improve error messages
* Be smart about the order that files are put onto a rom, to save space (this only matters with a significant alignment)
	* Then display available free space after creating a rom

* Improve this file, make it neater and well organized/prioritized
	* Have sections like refactoring, features, usage, ...
* Cow could probably be used a few more places to reduce allocations, but the potential benefits probably aren't worth the trouble. Look into this though
	* Wait a minute... Why don't I just add a name property or something to Segment?
* Add a way to check for free space (display this after rebuilding?)
* Add a way to specify a certain alignment for files matching a regex?
* Make Entry::new return a Result, not Option
* Add more subcommands
* Add a progress indicator for the rebuild command
* Put it on codereview.stackexchange.com
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like FST::new
* Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
* Make an API (it'd be a separate repo of course)
* Add an option for "info" to display values in hexadecimal (make a macro that accepts an option for hex output?)
* Add a command to disassemble an iso, plus an option to only disassemble a given dol file
* Add options or commands specifically for apploaders, dol files, etc... so the whole iso isn't needed
* Add a GCRebuilder compatibility mode (this'd just add weird extra zeros to some files, but that may not even be necessary idk)
* Improve the info subcommand output
* Anything in the source with a `TODO: ` label
* Make consts for the default system data path and the files in there
	* Maybe a property of Game that stores the filenames?
* Add offsets for other commands (info has it now)
* Add a way to only extract a single file
* Add tests
* Make sure it works for Japanese ROMs and unicode
* Add a way to make sure things are rebuilt in the right order
	* Pass in the sections that other section needs rather than open the file?

Refactoring
* Improve DisasmIter
* Be consistent (argument order, naming, output, and other stuff)
* Should FST be renamed to FileSystemTable?

This is golden:
http://www.gc-forever.com/forums/viewtopic.php?p=1487&sid=a5f89e4c4ee820c1305b27babf50eccd#p1487

