use std::fs::{create_dir, File};
use std::path::{Path, PathBuf};
use std::io::{self, BufReader, Seek, SeekFrom, Write};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::cmp;

use header::Header;
use app_loader::{APPLOADER_OFFSET, Apploader};
use dol::DOLHeader;
use fst::FST;
use fst::entry::{DirectoryEntry, Entry};
use ::{extract_section, WRITE_CHUNK_SIZE};

const ROM_SIZE: usize = 0x57058000;

use header::GAME_HEADER_SIZE;

#[derive(Debug)]
pub struct Game {
    pub header: Header,
    pub app_loader: Apploader,
    pub fst: FST,
    pub dol: DOLHeader,
    pub iso: BufReader<File>,
}

impl Game {

    pub fn open<P: AsRef<Path>>(filename: P) -> io::Result<Game> {
        let f = File::open(&filename)?;
        let mut iso = BufReader::new(f);

        let header = Header::new(&mut iso, 0)?;
        let fst = FST::new(&mut iso, header.fst_offset)?;
        let dol = DOLHeader::new(&mut iso, header.dol_offset)?;
        let app_loader = Apploader::new(&mut iso, APPLOADER_OFFSET)?;

        Ok(Game {
            header,
            app_loader,
            fst,
            dol,
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
        self.extract_game_header(&mut header_file)?;

        let mut fst_file = File::create(sys_data_path.join("Game.toc"))?;
        self.extract_fst(&mut fst_file)?;

        let mut apploader_file =
            File::create(sys_data_path.join("Apploader.ldr"))?;
        self.extract_app_loader(&mut apploader_file)?;

        let mut dol_file = File::create(sys_data_path.join("Start.dol"))?;
        self.extract_dol(&mut dol_file)?;

        self.extract_files(path.as_ref())?;
        Ok(())
    }

    pub fn extract_files<P>(&mut self, path: P) -> io::Result<usize>
        where P: AsRef<Path>
    {
        let count = self.fst.file_count;
        let res = self.fst.extract_filesystem(path, &mut self.iso, &|c|
            print!("\r{}/{} files written.", c, count)
        );
        println!();
        res
    }

    pub fn extract_game_header<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing game header...");
        self.iso.seek(SeekFrom::Start(0))?;
        extract_section(&mut self.iso, GAME_HEADER_SIZE, file)
    }

    // DOL is the format of the main executable on a GameCube disk
    pub fn extract_dol<W: Write>(&mut self, file: &mut W) -> io::Result<()> {
        println!("Writing DOL header...");
        DOLHeader::extract(&mut self.iso, self.header.dol_offset, file)
    }

    pub fn extract_app_loader<W>(&mut self, file: &mut W) -> io::Result<()>
        where W: Write
    {
        println!("Writing app loader...");
        Apploader::extract(&mut self.iso, file)
    }

    pub fn extract_fst<W: Write>(&mut self, file: &mut W) -> io::Result<()> {
        println!("Writing file system table...");
        FST::extract(&mut self.iso, file, self.header.fst_offset)
    }

    pub fn print_info(&self) {
        println!("Title: {}", self.header.title);
        println!("GameID: {}{}", self.header.game_code, self.header.maker_code);
        println!("FST offset: {}", self.header.fst_offset);
        println!("FST size: {} bytes", self.fst.size);
        println!("Main DOL offset: {}", self.header.dol_offset);
        println!("Main DOL entry point: {}", self.dol.entry_point);
        println!("Apploader size: {}", self.app_loader.total_size());

        println!("\nROM Layout:");
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
        regions.insert(self.header.dol_offset, (self.dol.dol_size, "Start.dol"));
        regions.insert(self.header.fst_offset, (self.fst.size, "Game.toc"));

        for (start, &(end, name)) in &regions {
            println!("{:#010x}-{:#010x}: {}", start, start + end as u64, name);
        }
    }

    pub fn rebuild_systemdata<P: AsRef<Path>>(root_path: P) -> io::Result<()> {
        // Apploader.ldr and Start.dol must exist before rebuilding Game.toc

        let fst_path = root_path.as_ref().join("&&systemdata/Game.toc");
        let mut fst_file = File::create(fst_path)?;
        FST::rebuild(root_path.as_ref())?.write(&mut fst_file)?;

        // Note: everything else must be rebuilt before the header can be,
        // and the old header must still exist

        let h = Header::rebuild(root_path.as_ref())?;
        let header_path = root_path.as_ref().join("&&systemdata/ISO.hdr");
        let mut header_file = File::create(header_path)?;
        h.write(&mut header_file)?;

        Ok(())
    }

    pub fn rebuild<P: AsRef<Path>, W: Write>(
        root_path: P,
        output: &mut W,
        rebuild_files: bool
    ) -> io::Result<()> {
        let mut bytes_written = 0;

        if rebuild_files {
            Game::rebuild_systemdata(root_path.as_ref())?;
        }

        let files = Game::make_sections_btree(root_path.as_ref())?;
        let total_files = files.len();

        for (i, (&offset, filename)) in files.iter().enumerate() {
            write_zeros((offset - bytes_written) as usize, output)?;
            bytes_written = offset;
            let mut file = File::open(root_path.as_ref().join(filename))?;
            let size = file.metadata()?.len();
            extract_section(&mut file, size as usize, output)?;
            bytes_written += size;
            print!("\r{}/{} files added.", i + 1, total_files);
        }
        println!();
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

        let mut header_buf = BufReader::new(File::open(&header_path)?);
        let header = Header::new(&mut header_buf, 0)?;
        let mut fst_buf = BufReader::new(File::open(&fst_path)?);
        let fst = FST::new(&mut fst_buf, 0)?;

        let mut tree = make_files_btree(&fst);
        tree.insert(0, header_path);
        tree.insert(APPLOADER_OFFSET, apploader_path);
        tree.insert(header.fst_offset, fst_path);
        tree.insert(header.dol_offset, dol_path);

        Ok(tree)
    }
}

fn write_zeros<W: Write>(count: usize, output: &mut W) -> io::Result<()> {
    lazy_static! {
        static ref ZEROS: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }
    let mut zeros = ZEROS.lock().unwrap();
    let block_size = cmp::min(count, WRITE_CHUNK_SIZE);
    zeros.resize(block_size, 0);
    for i in 0..(count / WRITE_CHUNK_SIZE + 1) {
        output.write_all(
            &zeros[..cmp::min(WRITE_CHUNK_SIZE, count - WRITE_CHUNK_SIZE * i)]
        )?;
    }
    Ok(())
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
            &Entry::Directory(ref sub_dir) => {
                fill_files_btree(
                    files,
                    sub_dir,
                    prefix.as_ref().join(&sub_dir.info.name),
                    fst
                );
            },
        };
    }
}

