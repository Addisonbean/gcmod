use std::io::{self, BufRead, Seek, SeekFrom, Write};
use std::fs::{File, create_dir_all};
use std::path::{Path, PathBuf};
use std::borrow::Cow;

use byteorder::{ReadBytesExt, BigEndian};

use layout_section::{LayoutSection, SectionType};
use ::extract_section;

pub const ENTRY_SIZE: usize = 12;

// writes in big endian
fn write_int_to_buffer(num: u64, buf: &mut [u8]) {
    for i in 0..buf.len() {
        buf[i] = ((num >> 8 * (buf.len() - i - 1)) & 0xff) as u8;
    }
}

#[derive(Debug)]
pub struct EntryInfo {
    pub index: usize,
    pub name: String,
    pub filename_offset: u64,

    // The fields below are not actually stored on the ROM:

    // This is the index of the directory that the entry is in.
    // For directories, this'll be the same as the parent_index field.
    pub directory_index: Option<usize>,
    pub full_path: PathBuf,
}

#[derive(Debug)]
pub struct FileEntry {
    pub info: EntryInfo,
    pub file_offset: u64,
    pub size: usize,
}

/*
 * `next_index` is the index of the next entry that's not in the directory.
 * For the root, this happens to be the amount of entries in the FST.
 * Also, `filename_offset` and `parent_index` are meaningless for the root
 */
#[derive(Debug)]
pub struct DirectoryEntry {
    pub info: EntryInfo,
    pub parent_index: usize,
    pub next_index: usize,
}

#[derive(Debug)]
pub enum Entry {
    File(FileEntry),
    Directory(DirectoryEntry),
}

impl Entry {
    pub fn new(
        entry: &[u8],
        index: usize,
        directory_index: Option<usize>
    ) -> Option<Entry> {
        // TODO: don't use unwrap when this is implemented
        // https://github.com/rust-lang/rfcs/issues/935
        let filename_offset =
            (&entry[1..4]).read_u24::<BigEndian>().unwrap() as u64;
        let f2 = (&entry[4..8]).read_u32::<BigEndian>().unwrap();
        let f3 = (&entry[8..12]).read_u32::<BigEndian>().unwrap();
        let name = String::new();
        let full_path = PathBuf::new();

        let info = EntryInfo {
            index,
            name,
            filename_offset,
            directory_index,
            full_path,
        };

        Some(match entry[0] {
            0 => Entry::File(FileEntry {
                info,
                file_offset: f2 as u64,
                size: f3 as usize,
            }),
            1 => Entry::Directory(DirectoryEntry {
                info,
                parent_index: f2 as usize,
                next_index: f3 as usize,
            }),
            _ => return None,
        })
    }

    pub fn write<W: Write>(&self, output: &mut W) -> io::Result<()> {
        let mut buf = [0; ENTRY_SIZE];
        let name_offset = self.info().filename_offset;
        if self.is_dir() { buf[0] = 1 }
        write_int_to_buffer(name_offset, &mut buf[1..4]);

        let (f2, f3) = match self {
            &Entry::File(ref e) => (e.file_offset, e.size as u64),
            &Entry::Directory(ref e) => (e.parent_index as u64, e.next_index as u64),
        };

        write_int_to_buffer(f2, &mut buf[4..8]);
        write_int_to_buffer(f3, &mut buf[8..12]);

        output.write_all(&buf[..])
    }

    pub fn info(&self) -> &EntryInfo {
        match self {
            &Entry::File(ref e) => &e.info,
            &Entry::Directory(ref e) => &e.info,
        }
    }

    pub fn info_mut(&mut self) -> &mut EntryInfo {
        match self {
            &mut Entry::File(ref mut e) => &mut e.info,
            &mut Entry::Directory(ref mut e) => &mut e.info,
        }
    }

    // move to Game?
    pub fn extract_with_name<P, R, F>(
        &self,
        filename: P,
        fst: &Vec<Entry>,
        iso: &mut R,
        callback: &F
    ) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
    {
        self.extract_with_name_and_count(filename, fst, iso, 0, callback)
    }

    pub fn extract_with_name_and_count<P, R, F>(
        &self,
        filename: P,
        fst: &Vec<Entry>,
        iso: &mut R,
        start_count: usize,
        callback: &F
    ) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
    {
        let mut count = start_count;
        match self {
            &Entry::Directory(ref d) => {
                create_dir_all(filename.as_ref())?;
                for e in d.iter_contents(fst) {
                    count += e.extract_with_name_and_count(
                        filename.as_ref().join(&e.info().name),
                        fst,
                        iso,
                        count,
                        callback,
                    )?;
                }
            },
            &Entry::File(ref f) => {
                let mut out = File::create(filename)?;
                f.extract(iso, &mut out)?;
                count += 1;
                callback(count);
            },
        }
        Ok(count - start_count)
    }

    pub fn read_filename<R: BufRead + Seek>(
        &mut self,
        reader: &mut R,
        str_tbl_addr: u64
    ) -> io::Result<()> {
        let info = self.info_mut();
        if info.index == 0 {
            info.name = "/".to_owned();
        } else {
            reader.seek(SeekFrom::Start(str_tbl_addr + info.filename_offset))?;
            // unsafe because the bytes read aren't guaranteed to be UTF-8
            unsafe {
                let mut bytes = info.name.as_mut_vec();
                reader.read_until(0, &mut bytes)?;
                // this gets rid of the trailing null byte
                bytes.pop();
            }
        }
        Ok(())
    }

    pub fn as_dir(&self) -> Option<&DirectoryEntry> {
        if let &Entry::Directory(ref dir) = self {
            Some(dir)
        } else {
            None
        }
    }

    pub fn as_file(&self) -> Option<&FileEntry> {
        if let &Entry::File(ref f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn as_dir_mut(&mut self) -> Option<&mut DirectoryEntry> {
        if let &mut Entry::Directory(ref mut dir) = self {
            Some(dir)
        } else {
            None
        }
    }

    pub fn as_file_mut(&mut self) -> Option<&mut FileEntry> {
        if let &mut Entry::File(ref mut f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn is_dir(&self) -> bool {
        self.as_dir().is_some()
    }

    pub fn is_file(&self) -> bool {
        self.as_file().is_some()
    }
}

impl FileEntry {
    // TODO: rename this
    pub fn extract<R, W>(&self, reader: &mut R, file: &mut W) -> io::Result<()>
        where R: BufRead + Seek, W: Write
    {
        reader.seek(SeekFrom::Start(self.file_offset))?;
        extract_section(reader, self.size, file)
    }
}

impl DirectoryEntry {
    pub fn iter_contents<'a>(
        &'a self,
        fst: &'a Vec<Entry>
    ) -> DirectoryIter<'a> {
        DirectoryIter::new(self, fst)
    }
}

pub struct DirectoryIter<'a> {
    dir: &'a DirectoryEntry,
    fst: &'a Vec<Entry>,
    current_index: usize,
}

impl<'a> DirectoryIter<'a> {
    fn new(dir: &'a DirectoryEntry, fst: &'a Vec<Entry>) -> DirectoryIter<'a> {
        DirectoryIter {
            dir,
            fst,
            current_index: dir.info.index + 1,
        }
    }
}

impl<'a> Iterator for DirectoryIter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<&'a Entry> {
        if self.current_index < self.dir.next_index {
            let res = &self.fst[self.current_index];
            let step = match res {
                &Entry::File(..) => 1,
                &Entry::Directory(ref d) => d.next_index - self.current_index,
            };
            self.current_index += step;
            Some(res)
        } else {
            None
        }
    }
}

impl<'a> LayoutSection<'a> for FileEntry {
    fn name(&'a self) -> Cow<'a, str> {
        self.info.full_path.to_string_lossy().into()
    }

    fn section_type(&self) -> SectionType {
        SectionType::File
    }

    fn len(&self) -> usize {
        self.size
    }

    fn start(&self) -> u64 {
        self.file_offset
    }

    fn print_info(&self) {
        println!("Path: {}", self.info.full_path.to_string_lossy());
        println!("Offset: {}", self.file_offset);
        println!("Size: {}", self.size);
    }
}

