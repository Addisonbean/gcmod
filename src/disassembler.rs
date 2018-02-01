use std::io::{self, BufReader, BufRead, Cursor, Error, Lines};
use std::ffi::OsStr;
use std::process::Command;
use std::str;

use byteorder::{BigEndian, ReadBytesExt};

pub struct Disassembler {
    objdump: Lines<BufReader<Cursor<Vec<u8>>>>,
}

impl Disassembler {
    pub fn new_with_objdump_path<P>(binary_path: P, objdump_path: P) -> io::Result<Disassembler>
        where P: AsRef<OsStr>
    {
        Disassembler::check_objdump_version(objdump_path.as_ref())?;
        let output = Command::new(objdump_path.as_ref())
            .args(&["-mpowerpc", "-D", "-b", "binary", "-EB", "-M", "750cl"])
            .arg(binary_path.as_ref())
            .output()?;
        if output.status.success() {
            let lines = BufReader::new(Cursor::new(output.stdout)).lines();
            let mut d = Disassembler { objdump: lines };
            d.advance_to_start()?;
            Ok(d)
        } else {
            Err(match output.status.code() {
                Some(c) => Error::new(io::ErrorKind::InvalidInput, &format!("The program `objdump` failed with a status of {}.", c)[..]),
                None => Error::new(io::ErrorKind::Interrupted, "The program `objdump` was interupted."),
            })
        }
    }

    pub fn new<P>(binary_path: P) -> io::Result<Disassembler>
        where P: AsRef<OsStr>
    {
        Disassembler::new_with_objdump_path(binary_path.as_ref(), "objdump".as_ref())
    }

    fn check_objdump_version<P>(objdump_path: P) -> io::Result<()>
        where P: AsRef<OsStr>
    {
        let expected_str = "GNU objdump";
        let yep = Command::new(objdump_path.as_ref())
            .arg("--version")
            .output().ok().map(|o| {
                let count = expected_str.len();
                o.stdout.len() >= count &&
                    str::from_utf8(&o.stdout[..count]) == Ok(expected_str)
            }) == Some(true);
        if yep {
            Ok(())
        } else {
            Err(Error::new(io::ErrorKind::InvalidInput, "GNU objdump required."))
        }
    }
    
    fn advance_to_start(&mut self) -> io::Result<()> {
        loop {
            match self.objdump.next() {
                Some(Ok(ref s)) if s == "00000000 <.data>:" => return Ok(()),
                Some(_) => (),
                None => return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid output from `objdump`.")),
            }
        }
    }
}

impl Iterator for Disassembler {
    type Item = Instruction;

    fn next(&mut self) -> Option<Instruction> {
        self.objdump.next().and_then(|line| {
            line.ok().and_then(|line| {
                let mut parts = line.split_whitespace()
                    .skip_while(|s| s.chars().find(|&c| c == ':').is_none()).skip(1);

                // This assumes every opcode is 4 bytes, is that right?
                // Aren't there exceptions?
                let mut bytes = Cursor::new((&mut parts).take(4).map(|s| 
                    u8::from_str_radix(s, 16).unwrap()
                ).collect::<Vec<_>>());

                bytes.read_u32::<BigEndian>().ok().map(|opcode| {
                    let text = parts.filter(|s|
                        !s.chars().any(char::is_whitespace)
                    ).collect::<Vec<_>>().join(" ");
                    Instruction { text, opcode }
                })
            })
        })
    }
}

#[derive(Debug)]
pub struct Instruction {
    text: String,
    opcode: u32,
}

