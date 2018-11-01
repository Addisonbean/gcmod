pub mod entry;

use std::borrow::Cow;
use std::cmp::max;
use std::collections::BTreeMap;
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

        // (parent_index, index of next file not in the parent dir, # of files in this parent)
        let mut parents = vec![(0, entry_count, 0)];

        for index in 1..entry_count {
            // Pop the directories that are no longer part of the current path
            while parents.last().map(|d| d.1) == Some(index) {
                if let Some((i, _, count)) = parents.pop() {
                    entries[i].as_dir_mut().unwrap().file_count = count;
                }
            }

            if let Some(p) = parents.last_mut() {
                p.2 += 1;
            }

            iso.take(ENTRY_SIZE as u64).read_exact(&mut entry_buffer)?;
            let e = Entry::new(&entry_buffer, index, parents.last().map(|d| d.0))?;
            match &e {
                Entry::File(f) => {
                    file_count += 1;
                    total_file_system_size += f.size;
                },
                Entry::Directory(d) => {
                    parents.push((index, d.next_index, 0));
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

        // Note: I'm not using `for e in &mut fst.entries`
        // because of borrow checking...
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

    pub fn entry_for_path(&self, path: impl AsRef<Path>) -> Option<&Entry> {
        let path = path.as_ref();
        if path.is_relative() {
            // Just treat the entire `path` like a single filename in this case
            self.entry_with_name(path, self.root())
        } else {
            // For each component in `path` (skipping the initial "/"),
            // try to find the corresponding file with that name
            path.iter().skip(1).try_fold(&self.entries[0], |entry, name| {
                entry.as_dir().and_then(|dir| {
                    dir.iter_contents(&self.entries).find(|e| &e.info().name[..] == name)
                })
            })
        }
    }

    fn entry_with_name<'a>(&'a self, name: impl AsRef<Path>, dir: &'a DirectoryEntry) -> Option<&'a Entry> {
        let name = name.as_ref();
        dir.iter_contents(&self.entries).find_map(|e| {
            if name.as_os_str() == &e.info().name[..] {
                Some(e)
            } else {
                e.as_dir().and_then(|subdir| self.entry_with_name(name, subdir))
            }
        })
    }

    pub fn get_parent_for_entry(&self, entry: &EntryInfo) -> Option<&Entry> {
        entry.directory_index.map(|i| &self.entries[i])
    }

    fn get_full_path(&self, entry: &EntryInfo) -> PathBuf {
        let mut parent = entry;
        let mut names = vec![&entry.name];

        while let Some(p) = self.get_parent_for_entry(parent) {
            parent = p.info();
            names.push(&parent.name);
        }

        names.iter().rev().collect()
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

