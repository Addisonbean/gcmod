pub mod entry;

use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{BigEndian, ReadBytesExt};

use self::entry::{Entry, ENTRY_SIZE};
use ::write_section;

pub const FST_OFFSET_OFFSET: u64 = 0x0424; 
pub const FST_SIZE_OFFSET: u64 = 0x0428;

#[derive(Debug)]
pub struct FST {
    /*
     * `file_count` is different from `entries.len()` in that
     * it doesn't include directories
     */
    pub file_count: usize,
    pub total_file_system_size: usize,
    pub entries: Vec<Entry>,
    pub size: usize,
}

impl FST {
    pub fn new<R>(iso: &mut R, fst_offset: u64) -> io::Result<FST>
        where R: BufRead + Seek
    {
        iso.seek(SeekFrom::Start(fst_offset))?;

        let mut entry_buffer: [u8; ENTRY_SIZE] = [0; ENTRY_SIZE];
        iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer)?;
        let root = Entry::new(&entry_buffer, 0)
            .expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir()
            .expect("Root fst wasn't a directory.")
            .next_index;

        let mut entries = Vec::with_capacity(entry_count);
        entries.push(root);

        let mut file_count = 0;
        let mut total_file_system_size = 0;

        for index in 1..entry_count {
            iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer)?;
            let e = Entry::new(&entry_buffer, index)
                .unwrap_or_else(||
                    panic!("Couldn't read fst entry {}.", index)
                );
            if let Some(f) = e.as_file() {
                file_count += 1;
                total_file_system_size += f.length;
            }
            entries.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0))?;

        for e in entries.iter_mut() {
            e.read_filename(iso, str_tbl_addr)?;
        }

        iso.seek(SeekFrom::Start(FST_SIZE_OFFSET))?;
        let size = iso.read_u32::<BigEndian>()? as usize;

        Ok(FST {
            file_count,
            total_file_system_size,
            entries,
            size,
        })
    }

    pub fn write_files<P, R, F>(
        &mut self, 
        path: P, 
        iso: &mut R, 
        callback: &F
    ) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
    {
        self.entries[0].write_with_name(path, &self.entries, iso, callback)
    }

    pub fn write_to_disk<R, W>(
        iso: &mut R,
        file: &mut W,
        fst_offset: u64
    ) -> io::Result<()>
        where R: Read + Seek, W: Write
    {
        iso.seek(SeekFrom::Start(FST_SIZE_OFFSET))?;
        let size = iso.read_u32::<BigEndian>()? as usize;

        iso.seek(SeekFrom::Start(fst_offset))?;
        write_section(iso, size, file)
    }
}

