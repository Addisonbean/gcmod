TODO
* Add a way to specify a custom objdump path
* Make Entry::new return a Result, not Option
* Add more subcommands
* Improve the cli options and subcommands, it could be more intuitive
* Put it on codereview.stackexchange.com
* Use expect more, unwrap less
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like FST::new
* Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
* Make an API (it'd be a separate repo of course)
* It's easy to call something like AppLoader::new forgetting to seek to the right spot first
* Add an option for "info" to display values in hexadecimal (make a macro that accepts an option for hex output?)
* Add a command to disassemble an iso, plus an option to only disassemble a given dol file
* Add options or commands specifically for apploaders, dol files, etc... so the whole iso isn't needed
* Add a GCRebuilder compatibility mode (this'd just add weird extra to some files, but that may not even be necessary idk)
* Improve the info subcommand output
* Add an option to rebuild Game.toc, the Start.dol header, and ISO.hdr to account for changes
	* Compress the contents/pack all the information to not waste any space, so there's more room for mods
* Anything in the source with a `TODO: ` label
* Make methods or functions for adding sections or files to a btree map, maybe make a struct to generalize the thing going on with make\_sections\_btree

Refactoring
* Only make stuff public if it needs to be
* Be consistent (argument order, naming, output, and other stuff)
* Rename stuff

