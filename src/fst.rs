use byteorder::{ReadBytesExt, BigEndian};
use std::io::{BufRead, Seek, SeekFrom};

#[derive(Debug)]
pub enum EntryType {
    File {
        file_offset: usize,
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
    pub filename_offset: usize,
    pub entry_type: EntryType,
}

impl Entry {
    pub fn new(entry: &[u8], index: usize) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        let filename_offset = (&entry[1..4]).read_u24::<BigEndian>().unwrap() as usize;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap() as usize;
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap() as usize;
        let name = String::new();

        let entry_type = match entry[0] {
            0 => EntryType::File {
                file_offset: f2,
                length: f3,
            },
            1 => EntryType::Directory {
                parent_index: f2,
                next_index: f3,
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

    pub fn read_filename<R: BufRead + Seek>(&mut self, reader: &mut R, str_tbl_addr: u64) {
        if self.index == 0 {
            self.name = "/".to_owned();
        } else {
            reader.seek(SeekFrom::Start(str_tbl_addr + self.filename_offset as u64)).unwrap();
            // unsafe because the bytes read aren't guaranteed to be UTF-8
            unsafe {
                let mut bytes = self.name.as_mut_vec();
                reader.read_until(0, &mut bytes).unwrap();
            }
        }
    }
}
