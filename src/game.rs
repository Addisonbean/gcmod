use std::fs::{create_dir, File};
use std::path::{Path, PathBuf};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::collections::BTreeMap;
use std::sync::Mutex;

use byteorder::{ReadBytesExt, BigEndian};

use header::Header;
use app_loader::{APPLOADER_OFFSET, Apploader};
use dol::{DOLHeader, DOL_OFFSET_OFFSET};
use fst::{FST, FST_OFFSET_OFFSET};
use fst::entry::{DirectoryEntry, Entry};
use ::write_section;

const ROM_SIZE: usize = 0x57058000;

use header::{
    GAMEID_SIZE, 
    GAMEID_OFFSET,
    GAME_HEADER_SIZE,
    TITLE_SIZE,
    TITLE_OFFSET
};

#[derive(Debug)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub app_loader: Apploader,
    pub fst: FST,
    pub dol: DOLHeader,
    pub fst_addr: u64,
    pub dol_addr: u64,
    pub iso: BufReader<File>,
}

impl Game {

    pub fn open<P: AsRef<Path>>(filename: P) -> io::Result<Game> {
        let f = File::open(&filename)?;
        let mut iso = BufReader::new(f);
        let mut game_id = String::with_capacity(GAMEID_SIZE);
        let mut title = String::with_capacity(TITLE_SIZE);

        iso.seek(SeekFrom::Start(GAMEID_OFFSET))?;
        iso.by_ref().take(GAMEID_SIZE as u64).read_to_string(&mut game_id)
            .unwrap();

        iso.seek(SeekFrom::Start(TITLE_OFFSET))?;
        iso.by_ref().take(TITLE_SIZE as u64).read_to_string(&mut title)
            .unwrap();

        // do some other stuff then:

        iso.seek(SeekFrom::Start(DOL_OFFSET_OFFSET))?;
        let dol_addr = iso.by_ref().read_u32::<BigEndian>()? as u64;

        iso.seek(SeekFrom::Start(FST_OFFSET_OFFSET))?;
        let fst_addr = iso.by_ref().read_u32::<BigEndian>()? as u64;

        let fst = FST::new(&mut iso, fst_addr)?;

        iso.seek(SeekFrom::Start(dol_addr))?;
        let dol = DOLHeader::new(&mut iso)?;

        let app_loader = Apploader::new(&mut iso)?;

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

    pub fn extract<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        // Not using `create_dir_all` here so it fails if `path` already exists.
        create_dir(path.as_ref())?;
        let sys_data_path = path.as_ref().join("&&systemdata");
        let sys_data_path: &Path = sys_data_path.as_ref();
        create_dir(sys_data_path)?;

        let mut header_file = File::create(sys_data_path.join("ISO.hdr"))?;
        self.write_game_header(&mut header_file)?;

        let mut fst_file = File::create(sys_data_path.join("Game.toc"))?;
        self.write_fst(&mut fst_file)?;

        let mut app_loader_file =
            File::create(sys_data_path.join("Apploader.ldr"))?;
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
    pub fn write_dol<W: Write>(&mut self, file: &mut W) -> io::Result<()> {
        println!("Writing DOL header...");
        DOLHeader::write_to_disk(&mut self.iso, self.dol_addr, file)
    }

    pub fn write_app_loader<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing app loader...");
        Apploader::write_to_disk(&mut self.iso, file)
    }

    pub fn write_fst<W: Write>(&mut self, file: &mut W) -> io::Result<()> {
        println!("Writing file system table...");
        FST::write_to_disk(&mut self.iso, file, self.fst_addr)
    }

    pub fn print_info(&self) {
        println!("Title: {}", self.title);
        println!("GameID: {}", self.game_id);
        println!("FST offset: {}", self.fst_addr);
        println!("FST size: {} bytes", self.fst.entries.len() * 12);
        println!("Main DOL offset: {}", self.dol_addr);
        println!("Main DOL entry point: {}", self.dol.entry_point);
        println!("Apploader size: {}", self.app_loader.total_size());

        self.print_layout();
    }

    pub fn print_layout(&self) {
        let mut regions = BTreeMap::new();

        // format: regions.insert(start, (size, name));
        regions.insert(0, (GAME_HEADER_SIZE, "ISO.hdr"));
        regions.insert(
            APPLOADER_OFFSET,
            (self.app_loader.total_size(), "Apploader.ldr")
        );
        regions.insert(self.dol_addr, (self.dol.dol_size, "Start.dol"));
        regions.insert(self.fst_addr, (self.fst.size, "Game.toc"));

        for (start, &(end, name)) in &regions {
            println!("{:#010x}-{:#010x}: {}", start, start + end as u64, name);
        }
    }

    pub fn rebuild<P, W>(root_path: P, output: &mut W) -> io::Result<()>
        where P: AsRef<Path>, W: Write
    {
        let mut bytes_written = 0;

        let files = Game::make_sections_btree(root_path.as_ref())?;

        for (&offset, filename) in &files {
            write_zeros((offset - bytes_written) as usize, output)?;
            bytes_written = offset;
            let mut file = File::open(root_path.as_ref().join(filename))?;
            let size = file.metadata()?.len();
            write_section(&mut file, size as usize, output)?;
            bytes_written += size;
        }
        write_zeros(ROM_SIZE - bytes_written as usize, output)
    }

    fn make_sections_btree<P>(root: P) -> io::Result<BTreeMap<u64, PathBuf>>
        where P: AsRef<Path>
    {
        let root = root.as_ref();
        let header_path = root.join("&&systemdata/ISO.hdr");
        let fst_path = root.join("&&systemdata/Game.toc");
        let apploader_path = root.join("&&systemdata/Apploader.ldr");
        let dol_path = root.join("&&systemdata/Start.dol");

        let header = Header::new(&mut File::open(&header_path)?)?;
        let fst = FST::new(&mut BufReader::new(File::open(&fst_path)?), 0)?;

        let mut tree = make_files_btree(&fst);
        tree.insert(0, header_path);
        tree.insert(APPLOADER_OFFSET, apploader_path);
        tree.insert(header.fst_addr, fst_path);
        tree.insert(header.dol_addr, dol_path);

        Ok(tree)
    }
}

fn write_zeros<W: Write>(count: usize, output: &mut W) -> io::Result<()> {
    lazy_static! {
        static ref ZEROS: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }
    let mut zeros = ZEROS.lock().unwrap();
    zeros.resize(count, 0);
    output.write_all(&zeros[..])
}

fn make_files_btree(fst: &FST) -> BTreeMap<u64, PathBuf> {
    let mut files = BTreeMap::new();
    fill_files_btree(&mut files, fst.entries[0].as_dir().unwrap(), "", fst);
    files
}

fn fill_files_btree<P: AsRef<Path>>(
    files: &mut BTreeMap<u64, PathBuf>,
    dir: &DirectoryEntry,
    prefix: P,
    fst: &FST
) {
    for entry in dir.iter_contents(&fst.entries) {
        match entry {
            &Entry::File(ref file) => {
                files.insert(
                    file.file_offset,
                    prefix.as_ref().join(&file.info.name)
                );
            },
            &Entry::Directory(ref dir) => {
                fill_files_btree(
                    files,
                    dir,
                    prefix.as_ref().join(&dir.info.name),
                    fst
                );
            },
        };
    }
}

