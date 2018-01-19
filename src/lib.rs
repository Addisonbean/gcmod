extern crate byteorder;

use std::io::{self, Read, Write};
use std::fs::File;
use std::path::Path;
use std::cmp::min;

mod game;
pub use game::Game;

pub mod fst;
pub mod dol;
pub mod app_loader;

const WRITE_CHUNK_SIZE: usize = 1048576; // 1048576 = 2^20 = 1MiB

pub fn write_section_to_file<R, P>(iso: &mut R, bytes: usize, path: P) -> io::Result<()>
    where R: Read, P: AsRef<Path>
{
    let mut f = File::create(path)?;

    let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
    let mut bytes_left = bytes;

    while bytes_left > 0 {
        let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

        let bytes_read = iso.take(bytes_to_read).read(&mut buf)?;
        if bytes_read == 0 { break }
        f.write_all(&buf[..bytes_read])?;

        bytes_left -= bytes_read;
    }

    Ok(())
}

