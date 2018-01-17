use std::fs::File;
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::cmp::{max, min};

use byteorder::{ReadBytesExt, BigEndian};

use fst::Entry;
use dol::Header as DOLHeader;
use app_loader::AppLoader;

// TODO: move these const's into modules
pub const WRITE_CHUNK_SIZE: usize = 1048576; // 1048576 = 2^20 = 1MiB

const GAMEID_SIZE: usize = 6;
const GAMEID_ADDR: u64 = 0;

// TODO: other sources suggest this size is larger, look into that...
const TITLE_SIZE: usize = 0x60;
const TITLE_ADDR: u64 = 0x20;

const DOL_ADDR_PTR: u64 = 0x0420; 

const FST_ADDR_PTR: u64 = 0x0424; 
const FST_ENTRY_SIZE: usize = 12;

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub fst: Vec<Entry>,
    pub file_count: usize,
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

        let mut entry_buffer: [u8; FST_ENTRY_SIZE] = [0; FST_ENTRY_SIZE];
        iso.seek(SeekFrom::Start(fst_addr)).unwrap();

        (&mut iso).take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
        let root = Entry::new(&entry_buffer, 0).expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir().expect("Root fst wasn't a directory.").next_index;

        let mut fst = Vec::with_capacity(entry_count);
        fst.push(root);

        let mut file_count = 0;

        for index in 1..entry_count {
            (&mut iso).take(FST_ENTRY_SIZE as u64).read_exact(&mut entry_buffer).unwrap();
            // fst.push(Entry::new(&entry_buffer, index).unwrap_or_else(|| panic!("Couldn't read fst entry {}.", index)));
            let e = Entry::new(&entry_buffer, index).unwrap_or_else(|| panic!("Couldn't read fst entry {}.", index));
            if e.is_file() { file_count += 1 }
            fst.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0)).unwrap();

        for e in fst.iter_mut() {
            e.read_filename(&mut iso, str_tbl_addr);
        }

        Some(Game {
            fst,
            game_id,
            title,
            file_count,
            fst_addr,
            dol_addr,
            iso,
        })
    }

    pub fn write_files<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        println!();
        let total = self.file_count;
        self.fst[0].write_with_name(path, &self.fst, &mut self.iso, &|c|
            print!("\r{}/{} files written.", c, total)
        ).map(|_| println!())
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

