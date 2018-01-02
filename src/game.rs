use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BigEndian};

use fst::FSTEntry;

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

