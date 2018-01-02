use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BigEndian};

#[derive(Debug)]
pub enum FSTEntry {
    File { filename_offset: usize, file_offset: usize, length: usize },

    /*
     * `next_index` is the index of the next entry that's not in the directory.
     * For the root, this happens to be the amount of entries in the FST
     */
    Directory { filename_offset: usize, parent_index: usize, next_index: usize },
}

impl FSTEntry {
    pub fn new(entry: &[u8]) -> Option<FSTEntry> {
        // TODO: don't use unwrap when this is implemented: https://github.com/rust-lang/rfcs/issues/935
        Some(match entry[0] {
            0 => FSTEntry::File {
                filename_offset: (&entry[1..4]).read_u24::<BigEndian>().unwrap() as usize,
                file_offset: (&entry[4..8]).read_u32::<BigEndian>().unwrap() as usize,
                length: (&entry[8..12]).read_u32::<BigEndian>().unwrap() as usize,
            },
            1 => FSTEntry::Directory {
                filename_offset: (&entry[1..4]).read_u24::<BigEndian>().unwrap() as usize,
                parent_index: (&entry[4..8]).read_u32::<BigEndian>().unwrap() as usize,
                next_index: (&entry[8..12]).read_u32::<BigEndian>().unwrap() as usize,
            },
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
}

impl Game {
    pub fn open(filename: &str) -> Option<Game> {
        let f = match File::open(&filename) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut the_reads = BufReader::new(f);
        let mut game_id = String::with_capacity(6);
        let mut title = String::with_capacity(0x60);

        (&mut the_reads).take(6).read_to_string(&mut game_id).unwrap();

        the_reads.seek(SeekFrom::Start(0x20)).unwrap();
        (&mut the_reads).take(0x60).read_to_string(&mut title).unwrap();

        // do some other stuff then:

        the_reads.seek(SeekFrom::Start(0x0424)).unwrap();
        let fst_addr = (&mut the_reads).read_u32::<BigEndian>().unwrap() as u64;

        let mut entry_buffer: [u8; 12] = [0; 12];
        the_reads.seek(SeekFrom::Start(fst_addr)).unwrap();

        (&mut the_reads).take(12).read_exact(&mut entry_buffer).unwrap();
        let root = FSTEntry::new(&entry_buffer);

        println!("{:?}", root);

        Some(Game {
            game_id,
            title,
        })
    }

}

