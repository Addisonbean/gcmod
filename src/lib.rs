extern crate byteorder;
#[macro_use]
extern crate lazy_static;

use std::io::{self, Read, Write};
use std::cmp::min;

mod game;
pub use game::Game;
pub use game::ROM_SIZE;

pub mod header;
pub mod fst;
pub mod dol;
pub mod apploader;
pub mod disassembler;
pub mod layout_section;

// 1048576 = 2^20 = 1MiB, there's no real good reason behind this choice
pub const WRITE_CHUNK_SIZE: usize = 1048576; 

// 32KiB
pub const DEFAULT_ALIGNMENT: u64 = 32 * 1024; 

pub fn extract_section<R: Read, W: Write>(
    iso: &mut R,
    bytes: usize,
    file: &mut W
) -> io::Result<()> {
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

pub fn align_to(n: u64, m: u64) -> u64 {
    ((n / m) + (if n % m == 0 { 0 } else { 1 })) * m
}

pub fn align(n: u64) -> u64 {
    align_to(n, DEFAULT_ALIGNMENT)
}

