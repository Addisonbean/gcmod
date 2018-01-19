use std::fs::File;
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::cmp::max;

use byteorder::{ReadBytesExt, BigEndian};

use ::write_section_to_file;
use app_loader::AppLoader;
use dol::DOL_OFFSET_OFFSET;
use fst::{FST, FST_OFFSET_OFFSET};

const GAMEID_SIZE: usize = 6;
const GAMEID_OFFSET: u64 = 0;

// TODO: other sources suggest this size is larger, look into that...
const TITLE_SIZE: usize = 0x60;
const TITLE_OFFSET: u64 = 0x20;

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub fst: FST,
    dol_addr: u64,
    fst_addr: u64,
    iso: BufReader<File>,
}

impl Game {

    // TODO: this needs to be split up into several functions
    pub fn open<P: AsRef<Path>>(filename: P) -> Option<Game> {
        let f = match File::open(&filename) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut iso = BufReader::new(f);
        let mut game_id = String::with_capacity(GAMEID_SIZE);
        let mut title = String::with_capacity(TITLE_SIZE);

        iso.seek(SeekFrom::Start(GAMEID_OFFSET)).unwrap();
        (&mut iso).take(GAMEID_SIZE as u64).read_to_string(&mut game_id).unwrap();

        iso.seek(SeekFrom::Start(TITLE_OFFSET)).unwrap();
        (&mut iso).take(TITLE_SIZE as u64).read_to_string(&mut title).unwrap();

        // do some other stuff then:

        iso.seek(SeekFrom::Start(DOL_OFFSET_OFFSET)).unwrap();
        let dol_addr = (&mut iso).read_u32::<BigEndian>().unwrap() as u64;

        iso.seek(SeekFrom::Start(FST_OFFSET_OFFSET)).unwrap();
        let fst_addr = (&mut iso).read_u32::<BigEndian>().unwrap() as u64;

        iso.seek(SeekFrom::Start(fst_addr)).unwrap();

        let fst = FST::new(&mut iso).unwrap();

        Some(Game {
            fst,
            game_id,
            title,
            fst_addr,
            dol_addr,
            iso,
        })
    }

    pub fn write_files<P: AsRef<Path>>(&mut self, path: P) -> io::Result<usize> {
        let count = self.fst.file_count;
        let res = self.fst.write_files(path, &mut self.iso, &|c|
            print!("\r{}/{} files written.", c, count)
        );
        println!("\n{} bytes written.", self.fst.total_size);
        res
    }

    // DOL is the format of the main executable on a GameCube disk
    pub fn write_dol<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let mut dol_size = 0;

        // 7 code segments
        for i in 0..7 {
            self.iso.seek(SeekFrom::Start(self.dol_addr + 0x00 + i * 4))?;
            let seg_offset = self.iso.read_u32::<BigEndian>()?;

            self.iso.seek(SeekFrom::Start(self.dol_addr + 0x90 + i * 4))?;
            let seg_size = self.iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        // 11 data segments
        for i in 0..11 {
            self.iso.seek(SeekFrom::Start(self.dol_addr + 0x1c + i * 4))?;
            let seg_offset = self.iso.read_u32::<BigEndian>()?;

            self.iso.seek(SeekFrom::Start(self.dol_addr + 0xac + i * 4))?;
            let seg_size = self.iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        self.iso.seek(SeekFrom::Start(self.dol_addr))?;

        write_section_to_file(&mut self.iso, dol_size as usize, path)
    }

    pub fn write_app_loader<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        AppLoader::write_to_disk(&mut self.iso, path)
    }
}

