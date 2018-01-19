use std::io::{self, BufRead, Seek, SeekFrom};
use std::fs::create_dir;
use std::path::Path;

use byteorder::{ReadBytesExt, BigEndian};

use ::write_section_to_file;

pub const ENTRY_SIZE: usize = 12;

#[derive(Debug)]
pub struct EntryInfo {
    pub index: usize,
    pub name: String,
    pub filename_offset: u64,
}

#[derive(Debug)]
pub struct FileEntry {
    pub info: EntryInfo,
    pub file_offset: u64,
    pub length: usize,
}

/*
 * `next_index` is the index of the next entry that's not in the directory.
 * For the root, this happens to be the amount of entries in the FST.
 * Also, `filename_offset` and `parent_index` are meaningless for the root
 */
#[derive(Debug)]
pub struct DirectoryEntry {
    pub info: EntryInfo,
    pub parent_index: usize,
    pub next_index: usize,
}

#[derive(Debug)]
pub enum Entry {
    File(FileEntry),
    Directory(DirectoryEntry),
}

impl Entry {
    pub fn new(entry: &[u8], index: usize) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        let filename_offset = (&entry[1..4]).read_u24::<BigEndian>().unwrap() as u64;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap();
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap();
        let name = String::new();

        let info = EntryInfo {
            index,
            name,
            filename_offset,
        };

        Some(match entry[0] {
            0 => Entry::File(FileEntry {
                info,
                file_offset: f2 as u64,
                length: f3 as usize,
            }),
            1 => Entry::Directory(DirectoryEntry {
                info,
                parent_index: f2 as usize,
                next_index: f3 as usize,
            }),
            _ => return None,
        })
    }

    pub fn info(&self) -> &EntryInfo {
        match self {
            &Entry::File(ref e) => &e.info,
            &Entry::Directory(ref e) => &e.info,
        }
    }

    pub fn info_mut(&mut self) -> &mut EntryInfo {
        match self {
            &mut Entry::File(ref mut e) => &mut e.info,
            &mut Entry::Directory(ref mut e) => &mut e.info,
        }
    }

    // move to Game?
    pub fn write_with_name<P, R, F>(&self, filename: P, fst: &Vec<Entry>, iso: &mut R, callback: &F) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
        // where P: AsRef<Path>, R: BufRead + Seek, F: Fn(&str, usize)
    {
        self.write_with_name_and_count(filename, fst, iso, 0, callback)
    }

    fn write_with_name_and_count<P, R, F>(&self, filename: P, fst: &Vec<Entry>, iso: &mut R, start_count: usize, callback: &F) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
        // where P: AsRef<Path>, R: BufRead + Seek, F: Fn(&str, usize)
    {
        let mut count = start_count;
        match self {
            &Entry::Directory(ref d) => {
                create_dir(filename.as_ref())?;
                for e in d.iter_contents(fst) {
                    count += e.write_with_name_and_count(filename.as_ref().join(&e.info().name), fst, iso, count, callback)?;
                }
            },
            &Entry::File(ref f) => {
                // let mut output = File::create(filename.as_ref())?;
                // f.write(iso, &mut output)?;
                f.write(iso, filename)?;
                count += 1;
                callback(count);
            },
        }
        Ok(count - start_count)
    }

    pub fn read_filename<R: BufRead + Seek>(&mut self, reader: &mut R, str_tbl_addr: u64) {
        let info = self.info_mut();
        if info.index == 0 {
            info.name = "/".to_owned();
        } else {
            reader.seek(SeekFrom::Start(str_tbl_addr + info.filename_offset)).unwrap();
            // unsafe because the bytes read aren't guaranteed to be UTF-8
            unsafe {
                let mut bytes = info.name.as_mut_vec();
                reader.read_until(0, &mut bytes).unwrap();
                bytes.pop();
            }
        }
    }

    pub fn as_dir(&self) -> Option<&DirectoryEntry> {
        if let &Entry::Directory(ref dir) = self {
            Some(dir)
        } else {
            None
        }
    }

    pub fn as_file(&self) -> Option<&FileEntry> {
        if let &Entry::File(ref f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn is_dir(&self) -> bool {
        self.as_dir().is_some()
    }

    pub fn is_file(&self) -> bool {
        self.as_file().is_some()
    }
}

impl FileEntry {
    // TODO: rename this
    pub fn write<R, P>(&self, reader: &mut R, filename: P) -> io::Result<()>
        where R: BufRead + Seek, P: AsRef<Path>
    {
        reader.seek(SeekFrom::Start(self.file_offset))?;
        write_section_to_file(reader, self.length, filename)
    }
}

impl DirectoryEntry {
    pub fn iter_contents<'a>(&'a self, fst: &'a Vec<Entry>) -> DirectoryIter<'a> {
        DirectoryIter::new(self, fst)
    }
}

pub struct DirectoryIter<'a> {
    dir: &'a DirectoryEntry,
    fst: &'a Vec<Entry>,
    current_index: usize,
}

impl<'a> DirectoryIter<'a> {
    fn new(dir: &'a DirectoryEntry, fst: &'a Vec<Entry>) -> DirectoryIter<'a> {
        DirectoryIter {
            dir,
            fst,
            current_index: dir.info.index + 1,
        }
    }
}

impl<'a> Iterator for DirectoryIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<&'a Entry> {
        if self.current_index < self.dir.next_index {
            let res = &self.fst[self.current_index];
            let step = match res {
                &Entry::File(..) => 1,
                &Entry::Directory(ref d) => d.next_index - self.current_index,
            };
            self.current_index += step;
            Some(res)
        } else {
            None
        }
    }
}

