extern crate byteorder;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::cmp::min;
use std::io::{self, Read, Write};

mod game;
pub use game::Game;
pub use game::ROM_SIZE;

pub mod apploader;
pub mod dol;
pub mod disassembler;
pub mod fst;
pub mod header;
pub mod layout_section;

// 1048576 = 2^20 = 1MiB, there's no real good reason behind this choice
pub const WRITE_CHUNK_SIZE: usize = 1048576; 

// 32KiB
pub const DEFAULT_ALIGNMENT: u64 = 32 * 1024; 

pub fn extract_section(
    mut iso: impl Read,
    bytes: usize,
    mut file: impl Write,
) -> io::Result<()> {
    let mut buf: [u8; WRITE_CHUNK_SIZE] = [0; WRITE_CHUNK_SIZE];
    let mut bytes_left = bytes;

    while bytes_left > 0 {
        let bytes_to_read = min(bytes_left, WRITE_CHUNK_SIZE) as u64;

        let bytes_read = (&mut iso).take(bytes_to_read).read(&mut buf)?;
        if bytes_read == 0 { break }
        file.write_all(&buf[..bytes_read])?;

        bytes_left -= bytes_read;
    }

    Ok(())
}

pub fn align_to(n: u64, m: u64) -> u64 {
    let extra = if n % m == 0 { 0 } else { 1 };
    ((n / m) + extra) * m
}

pub fn align(n: u64) -> u64 {
    align_to(n, DEFAULT_ALIGNMENT)
}

