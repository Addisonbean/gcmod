TODO

* Add traits like Rebuild, Info (or just use Display?), etc... (where would these be helpful? Anywhere?)
* Improve README.md, explain how to use it
* Warn if the iso to be rebuilt already exists
* Create an error if there isn't enough free space (and offer a suggestion like decreasing the alignment)
* Remove the offset param in Game::new
* Add cool stuff for getting info on the rom like a tree command to view the file system as a tree, an ls or list-files subcommand, cool stuff like that
* Add more types for the info subcommand, like dol, fst, and other stuff

* Cow could probably be used a few more places to reduce allocations, but the potential benefits probably aren't worth the trouble. Look into this though
	* Wait a minute... Why don't I just add a name property or something to Segment?
* Add a way to check for free space (display this after rebuilding?)
* Add a way to specify a certain alignment for files matching a regex?
	* Also a way to specify a different default alignment
* Keep working on the documentation on Google Docs
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
* Add a GCRebuilder compatibility mode (this'd just add weird extra to some files, but that may not even be necessary idk)
* Improve the info subcommand output
* Anything in the source with a `TODO: ` label
* Make methods or functions for adding sections or files to a btree map, maybe make a struct to generalize the thing going on with make\_sections\_btree
* Improve error messages
* Make consts for the default system data path and the files in there
	* Maybe a property of Game that stores the filenames?
* Add offsets for other commands (info has it now)
* Add an option to rebuild that doesn't rebuild the &&systemdata directory (should that be the default? probably?)
* Add a way to only extract a single file
* Accept hex numbers as inputs
* Add tests
* Make sure it works for Japanese ROMs and unicode

Refactoring
* Improve DisasmIter
* Only make stuff public if it needs to be
* Be consistent (argument order, naming, output, and other stuff)

This is golden:
http://www.gc-forever.com/forums/viewtopic.php?p=1487&sid=a5f89e4c4ee820c1305b27babf50eccd#p1487

