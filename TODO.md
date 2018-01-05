TODO
* Put it on codereview.stackexchange.com
* fst::Entry being en enum is annoying
* Use expect more, unwrap less
* More error handling, especially for corrupt isos
* Provide better documentation in the code, especially in methods like fst::Entry::new
* Implement Drop for Game to remove the temporary files created?
	* Add a property to disable this?
* How to read files on demand?
	* Keep a reference to the BufReader? Is that inefficient?
	* Read all the files then write them to disk?

tmp:

```
pub struct DirectoryEntry { ... }
pub struct FileEntry { ... }

pub trait EntryType {}
impl EntryType for DirectoryEntry {}
impl EntryType for FileEntry {}

struct Entry<E: EntryType> {
	...
	entry_type: E,
	...
}

impl Entry {
	...
}

impl Entry<FileEntry> {
	...
}

impl Entry<DirectoryEntry> {
	...
}

```

But then what type is Game::fst???

