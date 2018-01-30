use std::fs::{create_dir, File};
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, BigEndian};

use app_loader::AppLoader;
use dol::{Header as DOLHeader, DOL_OFFSET_OFFSET};
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
    pub dol: DOLHeader,
    pub fst_addr: u64,
    pub dol_addr: u64,
    pub iso: BufReader<File>,
}

impl Game {

    pub fn open<P>(filename: P) -> io::Result<Game>
        where P: AsRef<Path>
    {
        let f = File::open(&filename)?;
        let mut iso = BufReader::new(f);
        let mut game_id = String::with_capacity(GAMEID_SIZE);
        let mut title = String::with_capacity(TITLE_SIZE);

        iso.seek(SeekFrom::Start(GAMEID_OFFSET))?;
        (&mut iso).take(GAMEID_SIZE as u64).read_to_string(&mut game_id).unwrap();

        iso.seek(SeekFrom::Start(TITLE_OFFSET))?;
        (&mut iso).take(TITLE_SIZE as u64).read_to_string(&mut title).unwrap();

        // do some other stuff then:

        iso.seek(SeekFrom::Start(DOL_OFFSET_OFFSET))?;
        let dol_addr = (&mut iso).read_u32::<BigEndian>()? as u64;

        iso.seek(SeekFrom::Start(FST_OFFSET_OFFSET))?;
        let fst_addr = (&mut iso).read_u32::<BigEndian>()? as u64;

        iso.seek(SeekFrom::Start(fst_addr))?;

        let fst = FST::new(&mut iso)?;

        iso.seek(SeekFrom::Start(dol_addr))?;
        let dol = DOLHeader::new(&mut iso)?;

        Ok(Game {
            game_id,
            title,
            fst,
            dol,
            fst_addr,
            dol_addr,
            iso,
        })
    }

    pub fn extract<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        create_dir(path.as_ref())?;
        self.write_app_loader(path.as_ref().join("app_loader.bin"))?;
        self.write_dol(path.as_ref().join("boot.dol"))?;
        self.write_files(path.as_ref().join("file_system"))?;
        Ok(())
    }

    pub fn write_files<P>(&mut self, path: P) -> io::Result<usize>
        where P: AsRef<Path>
    {
        let count = self.fst.file_count;
        let res = self.fst.write_files(path, &mut self.iso, &|c|
            print!("\r{}/{} files written.", c, count)
        );
        println!("\n{} bytes written.", self.fst.total_size);
        res
    }

    // DOL is the format of the main executable on a GameCube disk
    pub fn write_dol<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        println!("Writing DOL header...");
        DOLHeader::write_to_disk(&mut self.iso, self.dol_addr, path)
    }

    pub fn write_app_loader<P>(&mut self, path: P) -> io::Result<()>
        where P: AsRef<Path>
    {
        println!("Writing app loader...");
        AppLoader::write_to_disk(&mut self.iso, path)
    }

    pub fn print_info(&self) {
        println!("Title: {}", self.title);
        println!("GameID: {}", self.game_id);
        println!("FST offset: {}", self.fst_addr);
        println!("FST size: {} bytes", self.fst.entries.len() * 12);
        println!("Main DOL offset: {}", self.dol_addr);
        println!("Main DOL size: {} bytes", self.dol.dol_size);
    }
}

