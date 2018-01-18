pub mod entry;

use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::path::Path;

use self::entry::Entry;

const FST_ENTRY_SIZE: usize = 12;

#[derive(Debug)]
pub struct FST {
    /*
     * `file_count` is different from `entries.len()` in that
     * it doesn't include directories
     */
    pub file_count: usize,
    // pub total_size: usize,
    pub entries: Vec<Entry>,
}

impl FST {
    pub fn new<R: BufRead + Seek>(iso: &mut R) -> io::Result<FST> {
        let mut entry_buffer: [u8; FST_ENTRY_SIZE] = [0; FST_ENTRY_SIZE];
        iso.take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
        let root = Entry::new(&entry_buffer, 0).expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir().expect("Root fst wasn't a directory.").next_index;

        let mut entries = Vec::with_capacity(entry_count);
        entries.push(root);

        let mut file_count = 0;

        for index in 1..entry_count {
            iso.take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
            let e = Entry::new(&entry_buffer, index).unwrap_or_else(|| panic!("Couldn't read fst entry {}.", index));
            if e.is_file() { file_count += 1 }
            entries.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0)).unwrap();

        for e in entries.iter_mut() {
            e.read_filename(iso, str_tbl_addr);
        }

        Ok(FST {
            file_count,
            entries,
        })
    }

    pub fn write_files<P, R>(&mut self, path: P, iso: &mut R) -> io::Result<()>
        where P: AsRef<Path>, R: BufRead + Seek
    {
        println!();
        let total = self.file_count;
        self.entries[0].write_with_name(path, &self.entries, iso, &|c|
            print!("\r{}/{} files written.", c, total)
        ).map(|_| println!())
    }
}

