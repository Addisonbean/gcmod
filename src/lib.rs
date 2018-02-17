extern crate byteorder;
#[macro_use]
extern crate lazy_static;

use std::io::{self, Read, Write};
use std::cmp::min;

mod game;
pub use game::Game;

pub mod header;
pub mod fst;
pub mod dol;
pub mod app_loader;
pub mod disassembler;

const WRITE_CHUNK_SIZE: usize = 1048576; // 1048576 = 2^20 = 1MiB

pub fn write_section<R, W>(iso: &mut R, bytes: usize, file: &mut W) -> io::Result<()>
    where R: Read, W: Write
{
    let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
    let mut bytes_left = bytes;

    while bytes_left > 0 {
        let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

        let bytes_read = iso.take(bytes_to_read).read(&mut buf)?;
        if bytes_read == 0 { break }
        file.write_all(&buf[..bytes_read])?;

        bytes_left -= bytes_read;
    }

    Ok(())
}

