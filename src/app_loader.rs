use std::io::{self, Read, Seek, SeekFrom, Write};

use byteorder::{ReadBytesExt, BigEndian};

use ::write_section;

pub const APP_LOADER_OFFSET: u64 = 0x2440;
const APP_LOADER_DATE_SIZE: usize = 10;
const APP_LOADER_ENTRY_POINT_ADDR: u64 = 0x2450;
const APP_LOADER_ENTRY_POINT_SIZE: u64 = 0xA0;
const APP_LOADER_SIZE_ADDR: u64 = 0x2454;

#[derive(Debug)]
pub struct AppLoader {
    pub date: String,
    pub entry_point: u64,
    pub size: usize,
    pub trailer_size: usize,
}

impl AppLoader {
    pub fn new<R: Read + Seek>(reader: &mut R) -> io::Result<AppLoader> {
        reader.seek(SeekFrom::Start(APP_LOADER_OFFSET))?;
        let mut date = String::new();
        reader.take(APP_LOADER_DATE_SIZE as u64).read_to_string(&mut date)?;
        
        reader.seek(SeekFrom::Current(6))?; // it's just fluff

        let entry_point = reader.read_u32::<BigEndian>()? as u64;
        let size = reader.read_u32::<BigEndian>()? as usize;
        let trailer_size = reader.read_u32::<BigEndian>()? as usize;

        Ok(AppLoader {
            date,
            entry_point,
            size,
            trailer_size,
        })
    }

    pub fn write_to_disk<R, W>(iso: &mut R, file: &mut W) -> io::Result<()>
        where R: Read + Seek, W: Write
    {
        iso.seek(SeekFrom::Start(APP_LOADER_SIZE_ADDR))?;
        let size = iso.read_u32::<BigEndian>()? as usize;
        iso.seek(SeekFrom::Start(APP_LOADER_OFFSET))?;
        write_section(iso, size, file)
    }
}

