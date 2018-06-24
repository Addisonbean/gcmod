use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::borrow::Cow;

use byteorder::{ReadBytesExt, BigEndian};

use layout_section::{LayoutSection, SectionType, UniqueLayoutSection, UniqueSectionType};
use ::{align_to, extract_section, ReadSeek};

pub const APPLOADER_OFFSET: u64 = 0x2440;
const APPLOADER_DATE_SIZE: usize = 0x0A;
const APPLOADER_ENTRY_POINT_ADDR: u64 = 0x2450;
const APPLOADER_ENTRY_POINT_SIZE: u64 = 0xA0;
const APPLOADER_SIZE_ADDR: u64 = 0x2454;

#[derive(Debug)]
pub struct Apploader {
    pub date: String,
    pub entry_point: u64,
    pub code_size: usize,
    pub trailer_size: usize,
}

impl Apploader {
    pub fn new<R: Read + Seek>(reader: &mut R, offset: u64) -> io::Result<Apploader> {
        reader.seek(SeekFrom::Start(offset))?;
        let mut date = String::new();
        reader.take(APPLOADER_DATE_SIZE as u64).read_to_string(&mut date)?;
        
        reader.seek(SeekFrom::Current(6))?; // it's just fluff

        let entry_point = reader.read_u32::<BigEndian>()? as u64;
        let code_size = reader.read_u32::<BigEndian>()? as usize;
        let trailer_size = reader.read_u32::<BigEndian>()? as usize;

        Ok(Apploader {
            date,
            entry_point,
            code_size,
            trailer_size,
        })
    }

    pub fn total_size(&self) -> usize {
        // self.code_size + self.trailer_size
        align_to((self.code_size + self.trailer_size) as u64, 32) as usize
    }

    pub fn extract<R, W>(iso: &mut R, file: &mut W) -> io::Result<()>
        where R: Read + Seek, W: Write
    {
        iso.seek(SeekFrom::Start(APPLOADER_SIZE_ADDR))?;
        let code_size = iso.read_u32::<BigEndian>()? as u64;
        let trailer_size = iso.read_u32::<BigEndian>()? as u64;
        iso.seek(SeekFrom::Start(APPLOADER_OFFSET))?;
        extract_section(iso, align_to(code_size + trailer_size, 32) as usize, file)
    }
}

impl<'a> LayoutSection<'a> for Apploader {
    fn name(&self) -> Cow<'static, str> {
        "&&systemdata/Apploader.hdr".into()
    }

    fn section_type(&self) -> SectionType {
        SectionType::Apploader
    }

    fn len(&self) -> usize {
        self.total_size()
    }

    fn start(&self) -> u64 {
        APPLOADER_OFFSET
    }

    fn print_info(&self) {
        println!("Offset: {}", APPLOADER_OFFSET);
        println!("Date: {}", self.date);
        println!("Code size: {} bytes", self.code_size);
        println!("Trailer size: {} bytes", self.trailer_size);
        println!("Entry point: not yet implemented");
        println!("Size (including code and trailer, aligned to 32 bytes): {}", self.total_size());
    }
}

impl<'a> UniqueLayoutSection<'a> for Apploader {
    fn section_type(&self) -> UniqueSectionType {
        UniqueSectionType::Apploader
    }

    fn with_offset(
        file: &mut BufReader<impl ReadSeek>,
        offset: u64,
    ) -> io::Result<Apploader> {
        Apploader::new(file, offset)
    }
}

