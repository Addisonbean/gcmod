use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::cmp::Ordering::*;
use std::borrow::Cow;

use header::Header;
use apploader::Apploader;
use dol::DOLHeader;
use fst::FST;

use ::extract_section;

pub trait LayoutSection<'a> {
    fn name(&'a self) -> Cow<'a, str>;

    fn section_type(&self) -> SectionType;

    fn print_info(&self);

    fn len(&self) -> usize;

    fn start(&self) -> u64;

    fn end(&self) -> u64 {
        self.start() + self.len() as u64 - 1
    }

    fn extract<R, W>(&self, mut iso: R, output: W) -> io::Result<()>
    where
        Self: Sized,
        R: Read + Seek,
        W: Write,
    {
        iso.seek(SeekFrom::Start(self.start()))?;
        extract_section(iso, self.len(), output)
    }

    fn compare_offset(&self, offset: u64) -> Ordering {
        if offset < self.start() {
            Less
        } else if offset > self.end() {
            Greater
        } else {
            Equal
        }
    }

    fn contains_offset(&self, offset: u64) -> bool {
        self.compare_offset(offset) == Equal
    }

    fn print_section_info(&'a self) {
        println!("Name: {}", self.name());
        println!("Type: {}", self.section_type().to_str());
        println!("Start: {}", self.start());
        println!("End: {}", self.end());
        println!("Size: {} bytes", self.len());
    }
}

impl<'a> PartialEq for LayoutSection<'a> {
    fn eq(&self, other: &LayoutSection) -> bool {
        self.start() == other.start()
    }
}

impl<'a> Eq for LayoutSection<'a> {}

impl<'a> PartialOrd for LayoutSection<'a> {
    fn partial_cmp(&self, other: &LayoutSection) -> Option<Ordering> {
        self.start().partial_cmp(&other.start())
    }
}

impl<'a> Ord for LayoutSection<'a> {
    fn cmp(&self, other: &LayoutSection) -> Ordering {
        self.start().cmp(&other.start())
    }
}

pub trait UniqueLayoutSection<'a>: LayoutSection<'a> {
    fn section_type(&self) -> UniqueSectionType;

    fn with_offset<R>(
        file: R,
        offset: u64,
    ) -> io::Result<Self>
    where
        Self: Sized,
        R: BufRead + Seek;
}


#[derive(Debug)]
pub enum SectionType {
    Header,
    Apploader,
    DOLHeader,
    FST,
    File,
    DOLSegment,
}

impl SectionType {
    pub fn to_str(&self) -> &'static str {
        use self::SectionType::*;
        match self {
            Header => "Header",
            Apploader => "Apploader",
            DOLHeader => "DOL Header",
            DOLSegment => "DOL Segment",
            FST => "File System Table",
            File => "File",
        }
    }
}

pub enum UniqueSectionType {
    Header,
    Apploader,
    DOLHeader,
    FST,
}

impl UniqueSectionType {
    pub fn to_str(&self) -> &'static str {
        use self::UniqueSectionType::*;
        match self {
            Header => "Header",
            Apploader => "Apploader",
            DOLHeader => "DOL Header",
            FST => "File System Table",
        }
    }

    pub fn with_offset<R>(
        &self,
        // file: impl BufRead,
        file: R,
        offset: u64,
    ) -> io::Result<Box<UniqueLayoutSection>>
        where Self: Sized,
              R: BufRead + Seek,
    {
        use self::UniqueSectionType as ST;
        match self {
            ST::Header => Header::with_offset(file, offset)
                .map(|s| Box::new(s) as Box<UniqueLayoutSection>),
            ST::Apploader => Apploader::with_offset(file, offset)
                .map(|s| Box::new(s) as Box<UniqueLayoutSection>),
            ST::DOLHeader => DOLHeader::with_offset(file, offset)
                .map(|s| Box::new(s) as Box<UniqueLayoutSection>),
            ST::FST => FST::with_offset(file, offset)
                .map(|s| Box::new(s) as Box<UniqueLayoutSection>),
        }
    }
}
