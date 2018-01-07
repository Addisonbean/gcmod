use std::fs::File;
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BigEndian};

use fst::Entry;

const GAMEID_SIZE: usize = 6;
const GAMEID_ADDR: u64 = 0;

const TITLE_SIZE: usize = 0x60;
const TITLE_ADDR: u64 = 0x20;

const FST_ADDR_PTR: u64 = 0x0424; 
const FST_ENTRY_SIZE: usize = 12;

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub fst: Vec<Entry>,
    iso: BufReader<File>,
}

impl Game {

    // TODO: this needs to be split up into several functions
    pub fn open(filename: &str) -> Option<Game> {
        let f = match File::open(&filename) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut iso = BufReader::new(f);
        let mut game_id = String::with_capacity(GAMEID_SIZE);
        let mut title = String::with_capacity(TITLE_SIZE);

        iso.seek(SeekFrom::Start(GAMEID_ADDR)).unwrap();
        (&mut iso).take(GAMEID_SIZE as u64).read_to_string(&mut game_id).unwrap();

        iso.seek(SeekFrom::Start(TITLE_ADDR)).unwrap();
        (&mut iso).take(TITLE_SIZE as u64).read_to_string(&mut title).unwrap();

        // do some other stuff then:

        iso.seek(SeekFrom::Start(FST_ADDR_PTR)).unwrap();
        let fst_addr = (&mut iso).read_u32::<BigEndian>().unwrap() as u64;

        let mut entry_buffer: [u8; FST_ENTRY_SIZE] = [0; FST_ENTRY_SIZE];
        iso.seek(SeekFrom::Start(fst_addr)).unwrap();

        (&mut iso).take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
        let root = Entry::new(&entry_buffer, 0).expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir().expect("Root fst wasn't a directory.").next_index;

        let mut fst = Vec::with_capacity(entry_count);
        fst.push(root);

        for index in 1..entry_count {
            (&mut iso).take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
            fst.push(Entry::new(&entry_buffer, index).unwrap_or_else(|| panic!("Couldn't read fst entry {}.", index));
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0)).unwrap();

        for e in fst.iter_mut() {
            e.read_filename(&mut iso, str_tbl_addr);
        }

        Some(Game {
            fst,
            game_id,
            title,
            iso,
        })
    }

    pub fn write_files<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.fst[0].write_with_name(path, &self.fst, &mut self.iso)
    }
}

