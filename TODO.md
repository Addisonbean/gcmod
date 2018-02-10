TODO
* Pass the right options to objdump so the offsets are correct
* Try making Segment take a generic, so the type is actually a type parameter (then add the derive default back)
	* Then clean up disassemble\_dol
* ...
* Add more subcommands
* Put it on codereview.stackexchange.com
* Use expect more, unwrap less
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like FST::new
* Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
* Make an API (it'd be a separate repo of course)
* It's easy to call something like AppLoader::new forgetting to seek to the right spot first
* Add an option for "info" to display values in hexadecimal
* Add a command to disassemble an iso, plus an option to only disassemble a given dol file
* Add options or commands specifically for apploaders, dol files, etc... so the whole iso isn't needed

Refactoring
* Only make stuff public if it needs to be
* Be consistent
* Rename stuff

