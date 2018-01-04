use byteorder::{ReadBytesExt, BigEndian};
use std::io::{BufRead, Seek, SeekFrom};

#[derive(Debug)]
pub enum Entry {
    File {
        index: usize,
        filename_offset: usize,
        file_offset: usize,
        length: usize,
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
    },
}

impl Entry {
    pub fn new(entry: &[u8], index: usize) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        let f1 = (&entry[1..4]).read_u24::<BigEndian>().unwrap() as usize;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap() as usize;
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap() as usize;
        Some(match entry[0] {
            0 => Entry::File {
                index, 
                filename_offset: f1,
                file_offset: f2,
                length: f3,
            },
            1 => Entry::Directory {
                index,
                filename_offset: f1,
                parent_index: f2,
                next_index: f3,
            },
            _ => return None,
        })
    }

    pub fn filename<R: BufRead + Seek>(&self, reader: &mut R, str_tbl_pos: u64) -> String {
        let (index, offset) = match self {
            &Entry::File { index, filename_offset, .. } => (index, filename_offset),
            &Entry::Directory { index, filename_offset, .. } => (index, filename_offset),
        };
        reader.seek(SeekFrom::Start(str_tbl_pos + offset as u64)).unwrap();

        let mut bytes = Vec::<u8>::new();
        reader.read_until(0, &mut bytes).unwrap();

        String::from_utf8(bytes).expect(&format!("Invalid (non-utf8) filename at index {}.\n\tstring table addr = {}\n\toffset = {}",
            index, str_tbl_pos, offset))
    }
}
