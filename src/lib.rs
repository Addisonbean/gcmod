extern crate byteorder;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::borrow::Cow;
use std::cmp::min;
use std::fmt;
use std::io::{self, Read, Write};
use std::num::ParseIntError;

mod game;
pub use game::Game;
pub use game::ROM_SIZE;

pub mod sections;

mod rom_rebuilder;
pub use rom_rebuilder::ROMRebuilder;

// 1048576 = 2^20 = 1MiB, there's no real good reason behind this choice
pub const WRITE_CHUNK_SIZE: usize = 1048576; 

// 32KiB
pub const DEFAULT_ALIGNMENT: u64 = 32 * 1024; 
pub const MIN_ALIGNMENT: u64 = 4;

pub mod paths {
    pub const APPLOADER_PATH: &'static str = "&&systemdata/Apploader.ldr";
    pub const DOL_PATH: &'static str = "&&systemdata/Start.dol";
    pub const FST_PATH: &'static str = "&&systemdata/Game.toc";
    pub const HEADER_PATH: &'static str = "&&systemdata/ISO.hdr";
}

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

pub fn align(n: u64, m: u64) -> u64 {
    let extra = if n % m == 0 { 0 } else { 1 };
    ((n / m) + extra) * m
}

#[derive(Copy, Clone)]
pub enum NumberStyle {
    Hexadecimal,
    Decimal,
}

// This isn't very efficient because the string returned will usually
// just get passed to another formatting macro, like println.
// The extra string allocation here isn't ideal, but it's not a problem
// at the moment. It'll need to be scrapped if this gets used in a place
// where it'll be called a lot.
pub fn format_u64(num: u64, style: NumberStyle) -> String {
    match style {
        NumberStyle::Hexadecimal => format!("{:#x}", num),
        NumberStyle::Decimal => format!("{}", num),
    }
}

pub fn format_usize(num: usize, style: NumberStyle) -> String {
    match style {
        NumberStyle::Hexadecimal => format!("{:#x}", num),
        NumberStyle::Decimal => format!("{}", num),
    }
}

pub fn parse_as_u64(text: &str) -> Result<u64, ParseIntError> {
    let is_hex = text.chars().count() > 2 && (&text[0..2] == "0x" || &text[0..2] == "0X");
    if is_hex {
        u64::from_str_radix(&text[2..], 16)
    } else {
        u64::from_str_radix(text, 10)
    }
}

pub fn parse_as_usize(text: &str) -> Result<usize, ParseIntError> {
    let is_hex = text.chars().count() > 2 && (&text[0..2] == "0x" || &text[0..2] == "0X");
    if is_hex {
        usize::from_str_radix(&text[2..], 16)
    } else {
        usize::from_str_radix(text, 10)
    }
}

pub struct AppError(Cow<'static, str>);

impl AppError {
    pub fn new(msg: impl Into<Cow<'static, str>>) -> AppError {
        AppError(msg.into())
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> AppError {
        AppError::new(e.to_string())
    }
}

pub type AppResult = Result<(), AppError>;
