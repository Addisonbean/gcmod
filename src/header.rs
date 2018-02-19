use std::io::{self, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ReadBytesExt};

use dol::DOL_OFFSET_OFFSET;
use fst::FST_OFFSET_OFFSET;

pub const GAME_HEADER_SIZE: usize = 0x2440;

pub const GAMEID_SIZE: usize = 6;
pub const GAMEID_OFFSET: u64 = 0;

// TODO: other sources suggest this size is larger, look into that...
pub const TITLE_SIZE: usize = 0x60;
pub const TITLE_OFFSET: u64 = 0x20;

pub struct Header {
    pub game_id: String,
    pub title: String,
    pub fst_addr: u64,
    pub dol_addr: u64,
}

impl Header {
    pub fn new<R: Read + Seek>(file: &mut R) -> io::Result<Header> {
        let mut game_id = String::with_capacity(GAMEID_SIZE);
        let mut title = String::with_capacity(TITLE_SIZE);

        file.seek(SeekFrom::Start(GAMEID_OFFSET))?;
        file.by_ref().take(GAMEID_SIZE as u64).read_to_string(&mut game_id)
            .unwrap();

        file.seek(SeekFrom::Start(TITLE_OFFSET))?;
        file.by_ref().take(TITLE_SIZE as u64).read_to_string(&mut title)
            .unwrap();

        file.seek(SeekFrom::Start(DOL_OFFSET_OFFSET))?;
        let dol_addr = file.read_u32::<BigEndian>()? as u64;

        file.seek(SeekFrom::Start(FST_OFFSET_OFFSET))?;
        let fst_addr = file.read_u32::<BigEndian>()? as u64;

        Ok(Header {
            game_id,
            title,
            fst_addr,
            dol_addr,
        })
    }
}

