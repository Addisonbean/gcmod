use std::io::{self, Read, Seek, SeekFrom, Write};

use byteorder::{BigEndian, ReadBytesExt};

use ::{
    align,
    extract_section,
    format_u64,
    format_usize,
    NumberStyle,
};

use sections::Section;

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
    pub fn new<R>(mut reader: R, offset: u64) -> io::Result<Apploader>
    where
        R: Read + Seek,
    {
        let reader = &mut reader;
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
        align((self.code_size + self.trailer_size) as u64, 32) as usize
    }

    pub fn extract<R, W>(mut iso: R, file: W) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        iso.seek(SeekFrom::Start(APPLOADER_SIZE_ADDR))?;
        let code_size = iso.read_u32::<BigEndian>()? as u64;
        let trailer_size = iso.read_u32::<BigEndian>()? as u64;
        iso.seek(SeekFrom::Start(APPLOADER_OFFSET))?;

        let aligned_size = align(code_size + trailer_size, 32);
        extract_section(iso, aligned_size as usize, file)
    }
}

impl Section for Apploader {
    fn print_info(&self, style: NumberStyle) {
        println!("Offset: {}", format_u64(APPLOADER_OFFSET, style));
        println!("Date: {}", self.date);
        println!("Code size: {} bytes", format_usize(self.code_size, style));
        println!("Trailer size: {} bytes", format_usize(self.trailer_size, style));
        println!("Entry point: not yet implemented");
        println!(
            "Size (including code and trailer, aligned to 32 bytes): {}",
            format_usize(self.total_size(), style),
        );
    }

    fn start(&self) -> u64 {
        APPLOADER_OFFSET
    }

    fn size(&self) -> usize {
        self.total_size()
    }
}
