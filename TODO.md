# TODO

## Next

* Have segments create their segment name in new, then make LayoutSection.name return a &str
* Add more methods to layout\_section, like rebuild and extract related stuff?

* Improve README.md, explain how to use it
* Warn if the iso to be rebuilt already exists
* Create an error if there isn't enough free space (and offer a suggestion like decreasing the alignment)
	* Also a way to specify a different default alignment
* Remove the offset param in Game::new
* Add cool stuff for getting info on the rom like a tree command to view the file system as a tree, an ls or list-files subcommand, cool stuff like that
* Improve the modules exported by lib.rs, like no more gamecube\_iso\_assistant::apploader::Apploader
* Add a "layout" options for the --type argument of the info subcommand
* Make methods or functions for adding sections or files to a btree map, maybe make a struct to generalize the thing going on with make\_sections\_btree
* Make a function like Game::is\_valid\_rom that checks to see if it has the magic byte and if it's the right size?
* Add an option to rebuild that doesn't rebuild the &&systemdata directory (should that be the default? probably?)
* Make the disassembler more practical/easier to use
	* Read the objdump path from an enviroment variable? As well as the option?
* Accept hex numbers as inputs
* Improve error messages

* Keep working on the documentation on Google Docs
* Improve this file, make it neater and well organized/prioritized
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
* Only make stuff public if it needs to be
* Be consistent (argument order, naming, output, and other stuff)
* Should FST be renamed to FileSystemTable?

This is golden:
http://www.gc-forever.com/forums/viewtopic.php?p=1487&sid=a5f89e4c4ee820c1305b27babf50eccd#p1487

