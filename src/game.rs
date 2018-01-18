use std::fs::File;
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::cmp::{max, min};

use byteorder::{ReadBytesExt, BigEndian};

use app_loader::AppLoader;
use fst::FST;

// TODO: move these const's into modules
pub const WRITE_CHUNK_SIZE: usize = 1048576; // 1048576 = 2^20 = 1MiB

const GAMEID_SIZE: usize = 6;
const GAMEID_ADDR: u64 = 0;

// TODO: other sources suggest this size is larger, look into that...
const TITLE_SIZE: usize = 0x60;
const TITLE_ADDR: u64 = 0x20;

const DOL_ADDR_PTR: u64 = 0x0420; 

const FST_ADDR_PTR: u64 = 0x0424; 

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

        iso.seek(SeekFrom::Start(GAMEID_ADDR)).unwrap();
        (&mut iso).take(GAMEID_SIZE as u64).read_to_string(&mut game_id).unwrap();

        iso.seek(SeekFrom::Start(TITLE_ADDR)).unwrap();
        (&mut iso).take(TITLE_SIZE as u64).read_to_string(&mut title).unwrap();

        // do some other stuff then:

        iso.seek(SeekFrom::Start(DOL_ADDR_PTR)).unwrap();
        let dol_addr = (&mut iso).read_u32::<BigEndian>().unwrap() as u64;

        iso.seek(SeekFrom::Start(FST_ADDR_PTR)).unwrap();
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

        let mut f = File::create(path)?;

        let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
        let mut bytes_left = dol_size as usize;

        while bytes_left > 0 {
            let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

            let bytes_read = (&mut self.iso).take(bytes_to_read).read(&mut buf)?;
            if bytes_read == 0 { break }
            f.write_all(&buf[..bytes_read])?;

            bytes_left -= bytes_read;
        }

        Ok(())
    }

    pub fn write_app_loader<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        AppLoader::write_to_disk(&mut self.iso, path)
    }
}

