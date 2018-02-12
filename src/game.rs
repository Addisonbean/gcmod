use std::fs::{create_dir, File};
use std::path::Path;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};

use byteorder::{ReadBytesExt, BigEndian};

use app_loader::AppLoader;
use dol::{Header as DOLHeader, DOL_OFFSET_OFFSET};
use fst::{FST, FST_OFFSET_OFFSET};
use ::write_section;

const GAME_HEADER_SIZE: usize = 0x2440;

const GAMEID_SIZE: usize = 6;
const GAMEID_OFFSET: u64 = 0;

// TODO: other sources suggest this size is larger, look into that...
const TITLE_SIZE: usize = 0x60;
const TITLE_OFFSET: u64 = 0x20;

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub app_loader: AppLoader,
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

        // iso.seek(SeekFrom::Start(fst_addr))?;
        let fst = FST::new(&mut iso, fst_addr)?;

        iso.seek(SeekFrom::Start(dol_addr))?;
        let dol = DOLHeader::new(&mut iso)?;

        let app_loader = AppLoader::new(&mut iso)?;

        Ok(Game {
            game_id,
            title,
            app_loader,
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
        // Don't use `create_dir_all` here so it fails if `path` already exists.
        create_dir(path.as_ref())?;
        let sys_data_path = path.as_ref().join("&&systemdata");
        let sys_data_path: &Path = sys_data_path.as_ref();
        create_dir(sys_data_path)?;

        let mut header_file = File::create(sys_data_path.join("ISO.hdr"))?;
        self.write_game_header(&mut header_file)?;

        let mut fst_file = File::create(sys_data_path.join("Game.toc"))?;
        self.write_fst(&mut fst_file)?;

        let mut app_loader_file = File::create(sys_data_path.join("AppLoader.ldr"))?;
        self.write_app_loader(&mut app_loader_file)?;

        let mut dol_file = File::create(sys_data_path.join("Start.dol"))?;
        self.write_dol(&mut dol_file)?;

        self.write_files(path.as_ref())?;
        Ok(())
    }

    pub fn write_files<P>(&mut self, path: P) -> io::Result<usize>
        where P: AsRef<Path>
    {
        let count = self.fst.file_count;
        let res = self.fst.write_files(path, &mut self.iso, &|c|
            print!("\r{}/{} files written.", c, count)
        );
        println!("\n{} bytes written.", self.fst.total_file_system_size);
        res
    }

    pub fn write_game_header<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing game header...");
        self.iso.seek(SeekFrom::Start(0))?;
        write_section(&mut self.iso, GAME_HEADER_SIZE, file)
    }

    // DOL is the format of the main executable on a GameCube disk
    pub fn write_dol<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing DOL header...");
        DOLHeader::write_to_disk(&mut self.iso, self.dol_addr, file)
    }

    pub fn write_app_loader<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing app loader...");
        AppLoader::write_to_disk(&mut self.iso, file)
    }

    pub fn write_fst<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing file system table...");
        FST::write_to_disk(&mut self.iso, file, self.fst_addr)
    }

    pub fn print_info(&self) {
        println!("Title: {}", self.title);
        println!("GameID: {}", self.game_id);
        println!("FST offset: {}", self.fst_addr);
        println!("FST size: {} bytes", self.fst.entries.len() * 12);
        println!("Main DOL offset: {}", self.dol_addr);
        println!("Main DOL entry point: {} bytes", self.dol.entry_point);
    }
}

