use std::cmp;
use std::cmp::Ordering::*;
use std::collections::BTreeMap;
use std::fs::{create_dir, File};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sections::apploader::{Apploader, APPLOADER_OFFSET};
use sections::dol::DOLHeader;
use sections::dol::segment::Segment;
use sections::fst::FST;
use sections::fst::entry::{DirectoryEntry, Entry};
use sections::header::{GAME_HEADER_SIZE, Header};
use sections::layout_section::{LayoutSection, UniqueLayoutSection, UniqueSectionType};
use ::{DEFAULT_ALIGNMENT, extract_section, format_u64, format_usize, NumberStyle, WRITE_CHUNK_SIZE};

pub const ROM_SIZE: usize = 0x57058000;

#[derive(Debug)]
pub struct Game {
    pub header: Header,
    pub apploader: Apploader,
    pub fst: FST,
    pub dol: DOLHeader,
}

impl Game {

    pub fn open<R>(mut iso: R, offset: u64) -> io::Result<Game>
    where
        R: BufRead + Seek,
    {
        let header = Header::new(&mut iso, offset)?;
        let apploader = Apploader::new(&mut iso, offset + APPLOADER_OFFSET)?;
        let dol = DOLHeader::new(&mut iso, offset + header.dol_offset)?;
        let fst = FST::new(&mut iso, offset + header.fst_offset)?;

        Ok(Game {
            header,
            apploader,
            fst,
            dol,
        })
    }

    pub fn rom_layout(&self) -> ROMLayout {
        let size = 5
            + self.dol.iter_segments().count()
            + self.fst.entries.len();

        let mut layout: Vec<&LayoutSection> = Vec::with_capacity(size);

        layout.push(&self.header);
        layout.push(&self.apploader);

        layout.push(&self.dol);

        for e in self.dol.iter_segments() {
            if e.size != 0 {
                layout.push(e);
            }
        }

        layout.push(&self.fst);

        for f in self.fst.entries.iter().filter_map(|e| e.as_file()) {
            layout.push(f);
        }

        layout.sort_unstable();

        ROMLayout(layout)
    }

    pub fn extract<R, P>(&mut self, mut iso: R, path: P) -> io::Result<()>
    where
        R: BufRead + Seek,
        P: AsRef<Path>,
    {
        // Not using `create_dir_all` here so it fails if `path` already exists.
        create_dir(path.as_ref())?;
        let sys_data_path = path.as_ref().join("&&systemdata");
        let sys_data_path: &Path = sys_data_path.as_ref();
        create_dir(sys_data_path)?;

        println!("Extracting system data...");

        let mut header_file = File::create(sys_data_path.join("ISO.hdr"))?;
        self.extract_game_header(&mut iso, &mut header_file)?;

        let mut fst_file = File::create(sys_data_path.join("Game.toc"))?;
        self.extract_fst(&mut iso, &mut fst_file)?;

        let mut apploader_file =
            File::create(sys_data_path.join("Apploader.ldr"))?;
        self.extract_apploader(&mut iso, &mut apploader_file)?;

        let mut dol_file = File::create(sys_data_path.join("Start.dol"))?;
        self.extract_dol(&mut iso, &mut dol_file)?;

        println!("Extracting file system...");

        self.extract_file_system(&mut iso, path.as_ref(), 4)?;
        Ok(())
    }

    pub fn extract_file_system(
        &mut self,
        iso: impl BufRead + Seek,
        path: impl AsRef<Path>,
        existing_files: usize,
    ) -> io::Result<usize> {
        let total = self.fst.file_count + existing_files;
        let mut count = existing_files;
        let res = self.fst.extract_file_system(path, iso, |_| {
            count += 1;
            print!("\r{}/{} files written.", count, total)
        });
        println!();
        res
    }

    pub fn extract_game_header(
        &mut self,
        iso: impl Read + Seek,
        file: impl Write,
    ) -> io::Result<()> {
        Header::extract(iso, file)
    }

    pub fn extract_dol<R, W>(&mut self, iso: R, file: W) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        DOLHeader::extract(iso, self.header.dol_offset, file)
    }

    pub fn extract_apploader<R, W>(&mut self, iso: R, file: W) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        Apploader::extract(iso, file)
    }

    pub fn extract_fst<R, W>(&mut self, iso: R, file: W) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        FST::extract(iso, file, self.header.fst_offset)
    }

    pub fn get_section_by_type(
        &self,
        section_type: &UniqueSectionType,
    ) -> &UniqueLayoutSection {
        use sections::layout_section::UniqueSectionType::*;
        match section_type {
            Header => &self.header as &UniqueLayoutSection,
            Apploader => &self.apploader as &UniqueLayoutSection,
            DOLHeader => &self.dol as &UniqueLayoutSection,
            FST => &self.fst as &UniqueLayoutSection,
        }
    }

    pub fn extract_section_with_name(
        &self,
        filename: impl AsRef<Path>,
        output: impl AsRef<Path>,
        iso: impl BufRead + Seek,
    ) -> io::Result<bool> {
        let output = output.as_ref();
        let filename = &*filename.as_ref().to_string_lossy();
        match filename {
            "&&systemdata/ISO.hdr" =>
                Header::extract(iso, &mut File::create(output)?).map(|_| true),
            "&&systemdata/Apploader.ldr" =>
                Apploader::extract(iso, &mut File::create(output)?)
                    .map(|_| true),
            "&&systemdata/Start.dol" =>
                DOLHeader::extract(
                    iso,
                    self.dol.offset,
                    &mut File::create(output)?,
                ).map(|_| true),
            "&&systemdata/Game.toc" =>
                FST::extract(iso, &mut File::create(output)?, self.fst.offset)
                    .map(|_| true),
            _ => {
                if let Some(e) = self.fst.entry_with_name(filename) {
                    e.extract_with_name(
                        output, &self.fst.entries,
                        iso,
                        &|_| {},
                    ).map(|_| true)
                } else if let Some((t, n)) =
                    Segment::parse_segment_name(filename)
                {
                    if let Some(s) = self.dol.find_segment(t, n) {
                        s.extract(iso, &mut File::create(output)?).map(|_| true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            },
        }
    }

    pub fn print_info(&self, style: NumberStyle) {
        println!("Title: {}", self.header.title);
        println!("GameID: {}{}", self.header.game_code, self.header.maker_code);
        println!("FST offset: {}", format_u64(self.header.fst_offset, style));
        println!("FST size: {} bytes", format_usize(self.fst.size, style));
        println!("Main DOL offset: {}", format_u64(self.header.dol_offset, style));
        println!("Main DOL entry point: {}", format_u64(self.dol.entry_point, style));
        println!("Apploader size: {}", format_usize(self.apploader.total_size(), style));

        println!("\nROM Layout:");
        self.print_layout();
    }

    pub fn print_layout(&self) {
        let mut regions = BTreeMap::new();

        // format: regions.insert(start, (size, name));
        regions.insert(0, (GAME_HEADER_SIZE, "ISO.hdr"));
        regions.insert(
            APPLOADER_OFFSET,
            (self.apploader.total_size(), "Apploader.ldr")
        );
        regions.insert(
            self.header.dol_offset,
            (self.dol.dol_size, "Start.dol")
        );
        regions.insert(self.header.fst_offset, (self.fst.size, "Game.toc"));

        for (start, &(end, name)) in &regions {
            println!("{:#010x}-{:#010x}: {}", start, start + end as u64, name);
        }
    }

    pub fn rebuild_systemdata(
        root_path: impl AsRef<Path>,
        alignment: u64,
    ) -> io::Result<()> {
        // Apploader.ldr and Start.dol must exist before rebuilding Game.toc

        let fst_path = root_path.as_ref().join("&&systemdata/Game.toc");
        let mut fst_file = File::create(fst_path)?;
        FST::rebuild(root_path.as_ref(), alignment)?.write(&mut fst_file)?;

        // Note: everything else must be rebuilt before the header can be,
        // and the old header must still exist

        let h = Header::rebuild(root_path.as_ref(), alignment)?;
        let header_path = root_path.as_ref().join("&&systemdata/ISO.hdr");
        let mut header_file = File::create(header_path)?;
        h.write(&mut header_file)?;

        Ok(())
    }

    pub fn rebuild(
        root_path: impl AsRef<Path>,
        mut output: impl Write,
        alignment: u64,
        rebuild_files: bool,
    ) -> io::Result<()> {
        let mut bytes_written = 0;

        if rebuild_files {
            Game::rebuild_systemdata(root_path.as_ref(), alignment)?;
        }

        let files = Game::make_sections_btree(root_path.as_ref())?;
        let total_files = files.len();

        for (i, &(offset, ref filename)) in files.iter().enumerate() {
            let mut file = File::open(filename)?;
            let size = file.metadata()?.len();
            if size == 0 { continue }

            write_zeros((offset - bytes_written) as usize, &mut output)?;
            bytes_written = offset;

            extract_section(&mut file, size as usize, &mut output)?;
            bytes_written += size;

            if bytes_written as usize > ROM_SIZE {
                println!();
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Error: not enough space. Try decreasing the file alignment with the -a option (the default is {} bytes).",
                        DEFAULT_ALIGNMENT,
                    ),
                ));
            }
            print!("\r{}/{} files added.", i + 1, total_files);
        }
        println!();
        write_zeros(ROM_SIZE - bytes_written as usize, &mut output)
    }

    fn make_sections_btree<P>(root: P) -> io::Result<Vec<(u64, PathBuf)>>
    where
        P: AsRef<Path>,
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

        let mut tree = make_files_btree(root, &fst);
        tree.push((0, header_path));
        tree.push((APPLOADER_OFFSET, apploader_path));
        tree.push((header.fst_offset, fst_path));
        tree.push((header.dol_offset, dol_path));
        tree.sort();

        Ok(tree)
    }
}

fn write_zeros(count: usize, mut output: impl Write) -> io::Result<()> {
    lazy_static! {
        static ref ZEROS: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }
    let mut zeros = ZEROS.lock().unwrap();
    let block_size = cmp::min(count, WRITE_CHUNK_SIZE);
    zeros.resize(block_size, 0);
    for i in 0..(count / WRITE_CHUNK_SIZE + 1) {
        (&mut output).write_all(
            &zeros[..cmp::min(WRITE_CHUNK_SIZE, count - WRITE_CHUNK_SIZE * i)]
        )?;
    }
    Ok(())
}

fn make_files_btree<P>(root: P, fst: &FST) -> Vec<(u64, PathBuf)>
where
    P: AsRef<Path>,
{
    let mut files = Vec::new();
    fill_files_btree(&mut files, fst.entries[0].as_dir().unwrap(), root, fst);
    files
}

fn fill_files_btree(
    files: &mut Vec<(u64, PathBuf)>,
    dir: &DirectoryEntry,
    prefix: impl AsRef<Path>,
    fst: &FST,
) {
    for entry in dir.iter_contents(&fst.entries) {
        match entry {
            Entry::File(ref file) => {
                files.push((
                    file.file_offset,
                    prefix.as_ref().join(&file.info.name),
                ));
            },
            Entry::Directory(ref sub_dir) => {
                fill_files_btree(
                    files,
                    sub_dir,
                    prefix.as_ref().join(&sub_dir.info.name),
                    fst,
                );
            },
        };
    }
}

// Use BinaryHeap?
pub struct ROMLayout<'a>(Vec<&'a LayoutSection<'a>>);

impl<'a> ROMLayout<'a> {
    pub fn find_offset(&'a self, offset: u64) -> Option<&'a LayoutSection<'a>> {
        // I don't use `Iterator::find` here because I can't break early once
        // a section is passed that has a greater starting offset than `offset`

        // Also, is there some builtin iterator or something that'll do this?
        // Probably...
        for s in &self.0 {
            match s.compare_offset(offset) {
                Less => return None,
                Equal => return Some(*s),
                Greater => (),
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

