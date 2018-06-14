use std::io::{self, Read, Seek, SeekFrom, Write};
use std::fmt;

use byteorder::{ReadBytesExt, BigEndian};

use layout_section::LayoutSection;
use ::{align_to, Extract, extract_section, ReadSeek};

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

impl<'a, 'b> From<&'b Apploader> for LayoutSection<'a, 'b> {
    fn from(a: &'b Apploader) -> LayoutSection<'a, 'b> {
        LayoutSection::new("&&systemdata/Apploader.ldr", "Apploader", APPLOADER_OFFSET, a.total_size(), a)
    }
}

impl Extract for Apploader {
    fn extract(&self, iso: &mut ReadSeek, output: &mut Write) -> io::Result<()> {
        iso.seek(SeekFrom::Start(APPLOADER_OFFSET))?;
        extract_section(iso, self.total_size(), output)
    }
}

impl fmt::Display for Apploader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Offset: {}", APPLOADER_OFFSET)?;
        writeln!(f, "Date: {}", self.date)?;
        writeln!(f, "Code size: {} bytes", self.code_size)?;
        writeln!(f, "Trailer size: {} bytes", self.trailer_size)?;
        writeln!(f, "Entry point: not yet implemented")?;
        write!(f, "Size (including code and trailer, aligned to 32 bytes): {}", self.total_size())
    }
}

