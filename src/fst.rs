use byteorder::{ReadBytesExt, BigEndian};
use std::io::{BufRead, Seek, SeekFrom};

#[derive(Debug)]
pub enum Entry {
    File {
        index: usize,
        filename_offset: usize,
        file_offset: usize,
        length: usize,
        name: String,
    },

    /*
     * `next_index` is the index of the next entry that's not in the directory.
     * For the root, this happens to be the amount of entries in the FST.
     * Also, `filename_offset` and `parent_index` are meaningless for the root
     */
    Directory {
        index: usize,
        filename_offset: usize,
        parent_index: usize,
        next_index: usize,
        name: String,
    },
}

impl Entry {
    pub fn new(entry: &[u8], index: usize) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        let f1 = (&entry[1..4]).read_u24::<BigEndian>().unwrap() as usize;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap() as usize;
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap() as usize;
        let name = String::new();
        Some(match entry[0] {
            0 => Entry::File {
                index, 
                filename_offset: f1,
                file_offset: f2,
                length: f3,
                name,
            },
            1 => Entry::Directory {
                index,
                filename_offset: f1,
                parent_index: f2,
                next_index: f3,
                name,
            },
            _ => return None,
        })
    }

    pub fn read_filename<R: BufRead + Seek>(&mut self, reader: &mut R, str_tbl_addr: u64) {
        let (index, offset, name) = match self {
            &mut Entry::File { index, filename_offset, ref mut name, .. } => (index, filename_offset, name),
            &mut Entry::Directory { index, filename_offset, ref mut name, .. } => (index, filename_offset, name),
        };
        /*
        let offset = match self {
            &Entry::File { filename_offset, .. } => filename_offset,
            &Entry::Directory { filename_offset, .. } => filename_offset,
        };
        */
        if index == 0 {
            // ya?
            // name = "/".to_owned();
            println!("ugh");
        } else {
            reader.seek(SeekFrom::Start(str_tbl_addr + offset as u64)).unwrap();
            // unsafe because the bytes read aren't guaranteed to be UTF-8
            unsafe {
                let mut bytes = name.as_mut_vec();
                reader.read_until(0, &mut bytes).unwrap();
            }
        }
    }
}
