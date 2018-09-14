pub mod entry;

use std::borrow::Cow;
use std::cmp::max;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use byteorder::{BigEndian, ReadBytesExt};

use sections::layout_section::{
    LayoutSection,
    SectionType,
    UniqueLayoutSection,
    UniqueSectionType
};
use ::{
    extract_section,
    format_u64,
    format_usize,
    NumberStyle,
    paths::FST_PATH,
};

use self::entry::{DirectoryEntry, Entry, EntryInfo, ENTRY_SIZE};

pub const FST_OFFSET_OFFSET: u64 = 0x0424; 
pub const FST_SIZE_OFFSET: u64 = 0x0428;

#[derive(Debug)]
pub struct FST {
    /*
     * `file_count` is different from `entries.len()` in that
     * it doesn't include directories
     */
    pub offset: u64,
    pub file_count: usize,
    pub total_file_system_size: usize,
    pub entries: Vec<Entry>,
    pub size: usize,
}

impl FST {
    pub fn new(mut iso: impl BufRead + Seek, offset: u64) -> io::Result<FST> {
        let mut iso = &mut iso;
        iso.seek(SeekFrom::Start(offset))?;

        let mut entry_buffer: [u8; ENTRY_SIZE] = [0; ENTRY_SIZE];
        iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer)?;
        let root = Entry::new(&entry_buffer, 0, None)
            .expect("Couldn't read root fst entry.");
        let entry_count = root.as_dir()
            .expect("Root fst wasn't a directory.")
            .next_index;

        let mut entries = Vec::with_capacity(entry_count);
        entries.push(root);

        let mut file_count = 0;
        let mut total_file_system_size = 0;

        // (parent_index, index of next file not in the parent dir)
        let mut parents = vec![(0, entry_count)];

        for index in 1..entry_count {
            while parents.last().map(|d| d.1) == Some(index) {
                parents.pop();
            }

            iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer)?;
            let e =
                Entry::new(&entry_buffer, index, parents.last().map(|d| d.0))
                .unwrap_or_else(||
                    panic!("Couldn't read fst entry {}.", index)
                );
            match &e {
                Entry::File(f) => {
                    file_count += 1;
                    total_file_system_size += f.size;
                },
                Entry::Directory(d) => {
                    parents.push((index, d.next_index));
                },
            }

            entries.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0))?;

        let mut end = 0;
        for e in entries.iter_mut() {
            e.read_filename(&mut iso, str_tbl_addr)?;

            let curr_end = iso.seek(SeekFrom::Current(0))?;
            end = max(curr_end, end);
        }

        let size = (end - offset) as usize;

        let mut fst = FST {
            offset,
            file_count,
            total_file_system_size,
            entries,
            size,
        };

        for i in 0..fst.entries.len() {
            let path = fst.get_full_path(fst.entries[i].info());
            fst.entries[i].info_mut().full_path = path;
        }

        Ok(fst)
    }

    pub fn root(&self) -> &DirectoryEntry {
        self.entries[0].as_dir().unwrap()
    }

    pub fn extract_file_system(
        &mut self, 
        path: impl AsRef<Path>,
        iso: impl BufRead + Seek,
        callback: impl FnMut(usize),
    ) -> io::Result<usize> {
        self.entries[0].extract_with_name(path, &self.entries, iso, callback)
    }

    pub fn extract(
        mut iso: impl Read + Seek,
        file: impl Write,
        fst_offset: u64,
    ) -> io::Result<()> {
        iso.seek(SeekFrom::Start(FST_SIZE_OFFSET))?;
        let size = iso.read_u32::<BigEndian>()? as usize;

        iso.seek(SeekFrom::Start(fst_offset))?;
        extract_section(iso, size, file)
    }

    pub fn write(&self, mut writer: impl Write) -> io::Result<()> {
        let mut sorted_names = BTreeMap::new();
        for e in &self.entries {
            e.write(&mut writer)?;
            sorted_names.insert(e.info().filename_offset, &e.info().name);
        }
        let null_byte = [0];
        for (_, name) in &sorted_names {
            (&mut writer).write(name.as_bytes())?;
            (&mut writer).write(&null_byte[..])?;
        }
        Ok(())
    }

    pub fn entry_with_name(&self, name: impl AsRef<Path>) -> Option<&Entry> {
        let name = name.as_ref().strip_prefix("/").unwrap_or(name.as_ref());

        let mut entry = &self.entries[0];
        for component in name.iter() {
            if let Some(dir) = entry.as_dir() {
                for e in dir.iter_contents(&self.entries) {
                    if component == OsStr::new(&e.info().name) {
                        entry = e;
                        break;
                    }
                }
            } else {
                return None;
            }
        }

        if name == Path::new(
            &entry.info().full_path.strip_prefix("/").unwrap()
        ) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn get_parent_for_entry(&self, entry: &EntryInfo) -> Option<&Entry> {
        entry.directory_index.map(|i| &self.entries[i])
    }

    fn get_full_path(&self, entry: &EntryInfo) -> PathBuf {
        let mut parent = entry;
        let mut names = vec![&entry.name];
        loop {
            parent = match self.get_parent_for_entry(parent) {
                Some(p) => p.info(),
                None => break,
            };

            names.push(&parent.name);
        }
        let mut path = PathBuf::new();
        for name in names.iter().rev() {
            path.push(name);
        }
        path
    }
}

impl<'a> LayoutSection<'a> for FST {
    fn name(&self) -> Cow<'static, str> {
        FST_PATH.into()
    }

    fn section_type(&self) -> SectionType {
        SectionType::FST
    }

    fn len(&self) -> usize {
        self.size
    }

    fn start(&self) -> u64 {
        self.offset
    }

    fn print_info(&self, style: NumberStyle) {
        println!("Offset: {}", format_u64(self.offset, style));
        println!("Total entries: {}", format_usize(self.entries.len(), style));
        println!("Total files: {}", format_usize(self.file_count, style));
        println!(
            "Total space used by files: {} bytes",
            format_usize(self.total_file_system_size, style),
        );
        println!("Size: {} bytes", format_usize(self.size, style));
    }
}

impl<'a> UniqueLayoutSection<'a> for FST {
    fn section_type(&self) -> UniqueSectionType {
        UniqueSectionType::FST
    }

    fn with_offset<R>(file: R, offset: u64) -> io::Result<FST>
    where
        Self: Sized,
        R: BufRead + Seek,
    {
        FST::new(file, offset)
    }
}

