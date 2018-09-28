use std::collections::BTreeMap;
use std::fs::{create_dir, File};
use std::io::{self, BufRead, Seek};
use std::path::Path;

use sections::apploader::{Apploader, APPLOADER_OFFSET};
use sections::dol::DOLHeader;
use sections::dol::segment::Segment;
use sections::fst::FST;
use sections::header::{GAME_HEADER_SIZE, Header};
use sections::layout_section::{LayoutSection, UniqueLayoutSection, UniqueSectionType};
use ::{
    format_u64,
    NumberStyle,
    paths::*,
};

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

        layout.extend(
            self.dol.iter_segments().map(|s| s as &dyn LayoutSection)
        );

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

        let header_file = File::create(sys_data_path.join("ISO.hdr"))?;
        Header::extract(&mut iso, header_file)?;

        let fst_file = File::create(sys_data_path.join("Game.toc"))?;
        FST::extract(&mut iso, fst_file, self.fst.offset)?;

        let apploader_file = File::create(sys_data_path.join("Apploader.ldr"))?;
        Apploader::extract(&mut iso, apploader_file)?;

        let mut dol_file = File::create(sys_data_path.join("Start.dol"))?;
        DOLHeader::extract(&mut iso, &mut dol_file, self.dol.offset)?;

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
            HEADER_PATH =>
                Header::extract(iso, &mut File::create(output)?).map(|_| true),
            APPLOADER_PATH =>
                Apploader::extract(iso, &mut File::create(output)?)
                    .map(|_| true),
            DOL_PATH =>
                DOLHeader::extract(
                    iso,
                    &mut File::create(output)?,
                    self.dol.offset,
                ).map(|_| true),
            FST_PATH =>
                FST::extract(iso, &mut File::create(output)?, self.fst.offset)
                    .map(|_| true),
            _ => {
                if let Some(e) = self.fst.entry_for_path(filename) {
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
        println!("Version: {}", format_u64(self.header.version as u64, style));

        println!("\nROM Layout:");
        self.print_layout();
    }

    pub fn print_layout(&self) {
        let mut regions = BTreeMap::new();

        // Format: regions.insert(start, (size, name));
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
}


// Use BinaryHeap?
pub struct ROMLayout<'a>(Vec<&'a LayoutSection<'a>>);

impl<'a> ROMLayout<'a> {
    pub fn find_offset(&'a self, offset: u64) -> Option<&'a LayoutSection<'a>> {
        self.0.binary_search_by(|s| s.compare_offset(offset)).ok()
            .map(|i| self.0[i])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

