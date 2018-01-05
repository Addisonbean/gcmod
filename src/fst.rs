use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::cmp::min;
use std::fs::{File, create_dir};
use std::path::Path;

use byteorder::{ReadBytesExt, BigEndian};

const WRITE_CHUNK_SIZE: usize = 16384; // 16384 = 2**14

#[derive(Debug)]
pub enum EntryType {
    File {
        file_offset: u64,
        length: usize,
    },

    /*
     * `next_index` is the index of the next entry that's not in the directory.
     * For the root, this happens to be the amount of entries in the FST.
     * Also, `filename_offset` and `parent_index` are meaningless for the root
     */
    Directory {
        parent_index: usize,
        next_index: usize,
    },
}

#[derive(Debug)]
pub struct Entry {
    pub index: usize,
    pub name: String,
    pub filename_offset: u64,
    pub entry_type: EntryType,
}

impl Entry {
    pub fn new(entry: &[u8], index: usize) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        let filename_offset = (&entry[1..4]).read_u24::<BigEndian>().unwrap() as u64;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap();
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap();
        let name = String::new();

        let entry_type = match entry[0] {
            0 => EntryType::File {
                file_offset: f2 as u64,
                length: f3 as usize,
            },
            1 => EntryType::Directory {
                parent_index: f2 as usize,
                next_index: f3 as usize,
            },
            _ => return None,
        };

        Some(Entry {
            index,
            name,
            filename_offset,
            entry_type,
        })
    }

    // TODO: return a result?
    pub fn read_filename<R: BufRead + Seek>(&mut self, reader: &mut R, str_tbl_addr: u64) {
        if self.index == 0 {
            self.name = "/".to_owned();
        } else {
            reader.seek(SeekFrom::Start(str_tbl_addr + self.filename_offset)).unwrap();
            // unsafe because the bytes read aren't guaranteed to be UTF-8
            unsafe {
                let mut bytes = self.name.as_mut_vec();
                reader.read_until(0, &mut bytes).unwrap();
                bytes.pop();
            }
        }
    }

    // TODO: return a result (io::Result? ya?) (also, does this have to be BufRead, rather than Read? Should it?
    pub fn write<R, W>(&self, reader: &mut R, writer: &mut W) -> io::Result<()>
        where R: BufRead + Seek, W: Write
    {
        match self.entry_type {
            EntryType::File { length, file_offset } => {
                reader.seek(SeekFrom::Start(file_offset))?;
                let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
                let mut bytes_left = length;

                while bytes_left > 0 {
                    let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

                    let bytes_read = reader.take(bytes_to_read).read(&mut buf)?;
                    writer.write_all(&buf[..bytes_read])?;

                    buf = [0; WRITE_CHUNK_SIZE];
                    bytes_left -= bytes_read;
                }
            },
            // EntryType::Directory { .. } => unimplemented!(),
            EntryType::Directory {..} => (), // TODO: throw error? make the directory?
        };
        Ok(())
    }

    // move to Game?
    pub fn write_with_name<P: AsRef<Path>, R: BufRead + Seek>(&self, filename: P, fst: &Vec<Entry>, iso: &mut R) -> io::Result<()> {
        match self.entry_type {
            EntryType::Directory {..} => {
                create_dir(filename.as_ref())?;
                for e in self.iter_contents(fst) {
                    e.write_with_name(filename.as_ref().join(&e.name), fst, iso)?;
                }
            },
            EntryType::File {..} => {
                let mut f = File::create(filename.as_ref())?;
                self.write(iso, &mut f);
            },
        }
        Ok(())
    }

    pub fn iter_contents<'a>(&'a self, fst: &'a Vec<Entry>) -> DirectoryIter<'a> {
        if let EntryType::Directory { .. } = self.entry_type {
            DirectoryIter::new(self, fst)
        } else {
            // this panic will be removed in the next commit
            panic!()
        }
    }
}

pub struct DirectoryIter<'a> {
    entry: &'a Entry,
    fst: &'a Vec<Entry>,
    current_index: usize,
}

impl<'a> DirectoryIter<'a> {
    fn new(entry: &'a Entry, fst: &'a Vec<Entry>) -> DirectoryIter<'a> {
        DirectoryIter {
            entry,
            fst,
            current_index: entry.index + 1,
        }
    }
}

impl<'a> Iterator for DirectoryIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<&'a Entry> {
        if let EntryType::Directory { next_index, .. } = self.entry.entry_type {
            if self.current_index < next_index {
                let res = &self.fst[self.current_index];
                let step = match res.entry_type {
                    EntryType::File { .. } => 1,
                    EntryType::Directory { next_index: _next_index, .. } => _next_index - self.current_index,
                };
                self.current_index += step;
                return Some(res);
            }
        }
        None
    }
}

