TODO
* Make Entry::new return a Result, not Option
* Add an option to rebuild that doesn't rebuild the &&systemdata directory
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
* Add an argument to the `info -t header` command to control the offset read in the file
	* Add offsets for other commands too
* Add an option not to rebuild any of the files (should that be the default? probably?)

Refactoring
* Only make stuff public if it needs to be
* Be consistent (argument order, naming, output, and other stuff)
* Rename stuff (apploader is still app\_loader is a few places, include the app\_loader filename itself, filename is sometimes file\_name)

