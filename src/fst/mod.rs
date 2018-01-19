pub mod entry;

use std::io::{self, BufRead, Read, Seek, SeekFrom};
use std::path::Path;

use self::entry::Entry;

use self::entry::ENTRY_SIZE;
pub const FST_OFFSET_OFFSET: u64 = 0x0424; 

#[derive(Debug)]
pub struct FST {
    /*
     * `file_count` is different from `entries.len()` in that
     * it doesn't include directories
     */
    pub file_count: usize,
    pub total_size: usize,
    pub entries: Vec<Entry>,
}

impl FST {
    pub fn new<R: BufRead + Seek>(iso: &mut R) -> io::Result<FST> {
        let mut entry_buffer: [u8; ENTRY_SIZE] = [0; ENTRY_SIZE];
        iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
        let root = Entry::new(&entry_buffer, 0).expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir().expect("Root fst wasn't a directory.").next_index;

        let mut entries = Vec::with_capacity(entry_count);
        entries.push(root);

        let mut file_count = 0;
        let mut total_size = 0;

        for index in 1..entry_count {
            iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
            let e = Entry::new(&entry_buffer, index).unwrap_or_else(|| panic!("Couldn't read fst entry {}.", index));
            if let Some(f) = e.as_file() {
                file_count += 1;
                total_size += f.length;
            }
            entries.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0)).unwrap();

        for e in entries.iter_mut() {
            e.read_filename(iso, str_tbl_addr);
        }

        Ok(FST {
            file_count,
            total_size,
            entries,
        })
    }

    pub fn write_files<P, R, F>(&mut self, path: P, iso: &mut R, callback: &F) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
    {
        self.entries[0].write_with_name(path, &self.entries, iso, callback)
    }
}

