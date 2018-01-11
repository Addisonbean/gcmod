TODO
* Put it on codereview.stackexchange.com
* Use expect more, unwrap less
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like fst::Entry::new
* Wouldn't it be more efficient to pass the iso BufReader directly to Entry::new, rather than copying it to an Array and passing that?
* Use u32 instead of u64?
* Only make this public if they need to be

