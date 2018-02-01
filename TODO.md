TODO
* Add more subcommands
* Put it on codereview.stackexchange.com
* Use expect more, unwrap less
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like FST::new
* Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
* Make an API (it'd be a separate repo of course)
* It's easy to call something like AppLoader::new forgetting to seek to the right spot first
* Add an option for "info" to display values in hexadecimal

Refactoring
* Only make stuff public if it needs to be
* Be consistent
* Rename stuff
* main.rs has some very repetitive code. Move stuff into functions. Make it look nice

