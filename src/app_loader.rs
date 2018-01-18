use std::io::{self, Read, Seek, SeekFrom, Write};
use std::fs::File;
use std::path::Path;
use std::cmp::min;

use byteorder::{ReadBytesExt, BigEndian};

use ::WRITE_CHUNK_SIZE;

pub const APP_LOADER_ADDR: u64 = 0x2440;
pub const APP_LOADER_DATE_SIZE: usize = 10;
pub const APP_LOADER_ENTRY_POINT_ADDR: u64 = 0x2450;
pub const APP_LOADER_ENTRY_POINT_SIZE: u64 = 0xA0;
pub const APP_LOADER_SIZE_ADDR: u64 = 0x2454;

#[derive(Debug)]
pub struct AppLoader {
    pub date: String,
    pub entry_point: u32,
    pub size: usize,
    pub trailer_size: usize,
    // code: Vec<u8>,
}

impl AppLoader {
    pub fn new<R: Read + Seek>(reader: &mut R) -> io::Result<AppLoader> {
        let mut date = String::new();
        reader.take(APP_LOADER_DATE_SIZE as u64).read_to_string(&mut date)?;
        
        reader.seek(SeekFrom::Current(6))?; // it's just fluff

        let entry_point = reader.read_u32::<BigEndian>()?;
        let size = reader.read_u32::<BigEndian>()? as usize;
        let trailer_size = reader.read_u32::<BigEndian>()? as usize;

        Ok(AppLoader {
            date,
            entry_point,
            size,
            trailer_size,
        })
    }

    pub fn write_to_disk<R, P>(iso: &mut R, path: P) -> io::Result<()>
        where R: Read + Seek, P: AsRef<Path>
    {
        iso.seek(SeekFrom::Start(APP_LOADER_SIZE_ADDR))?;
        let size = iso.read_u32::<BigEndian>()?;
        iso.seek(SeekFrom::Start(APP_LOADER_ADDR))?;

        let mut f = File::create(path)?;

        let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
        let mut bytes_left = size as usize;

        while bytes_left > 0 {
            let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

            let bytes_read = iso.take(bytes_to_read).read(&mut buf)?;
            if bytes_read == 0 { break }
            f.write_all(&buf[..bytes_read])?;

            bytes_left -= bytes_read;
        }

        Ok(())
    }
}

