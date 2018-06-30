pub mod entry;

use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::fs::{File, read_dir};
use std::ffi::OsStr;
use std::collections::BTreeMap;
use std::borrow::Cow;
use std::cmp::max;

use byteorder::{BigEndian, ReadBytesExt};

use self::entry::{DirectoryEntry, Entry, EntryInfo, FileEntry, ENTRY_SIZE};
use apploader::APPLOADER_OFFSET;
use layout_section::{LayoutSection, SectionType, UniqueLayoutSection, UniqueSectionType};
use ::{align, extract_section, ReadSeek};

pub const FST_OFFSET_OFFSET: u64 = 0x0424; 
pub const FST_SIZE_OFFSET: u64 = 0x0428;

struct RebuildInfo {
    entries: Vec<Entry>,
    file_offset: u64,
    filename_offset: u64,
    file_count: usize,
    parent_index: Option<usize>,
    current_path: PathBuf,
}

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
    pub fn new<R>(iso: &mut R, offset: u64) -> io::Result<FST>
        where R: BufRead + Seek
    {
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
                    // parent_dirs.push((index, d.next_index - index - 1));
                    parents.push((index, d.next_index));
                },
            }

            entries.push(e);
        }

        let str_tbl_addr = iso.seek(SeekFrom::Current(0))?;

        let mut end = 0;
        for e in entries.iter_mut() {
            e.read_filename(iso, str_tbl_addr)?;

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

    pub fn extract_filesystem<P, R, F>(
        &mut self, 
        path: P, 
        iso: &mut R, 
        callback: &F
    ) -> io::Result<usize>
        where P: AsRef<Path>, R: BufRead + Seek, F: Fn(usize)
    {
        self.entries[0].extract_with_name(path, &self.entries, iso, callback)
    }

    pub fn extract<R, W>(
        iso: &mut R,
        file: &mut W,
        fst_offset: u64
    ) -> io::Result<()>
        where R: Read + Seek, W: Write
    {
        iso.seek(SeekFrom::Start(FST_SIZE_OFFSET))?;
        let size = iso.read_u32::<BigEndian>()? as usize;

        iso.seek(SeekFrom::Start(fst_offset))?;
        extract_section(iso, size, file)
    }

    pub fn rebuild<P: AsRef<Path>>(root_path: P) -> io::Result<FST> {
        let ldr_path = root_path.as_ref().join("&&systemdata/Apploader.ldr");
        let appldr_size = File::open(ldr_path)?.metadata()?.len();

        let dol_path = root_path.as_ref().join("&&systemdata/Start.dol");
        let dol_size = File::open(dol_path)?.metadata()?.len() as u64;

        // ISO layout
        // Header -> apploader -> fst -> dol -> fs

        let root_entry = Entry::Directory(DirectoryEntry {
            info: EntryInfo {
                index: 0,
                name: "/".to_owned(),
                filename_offset: 0,
                directory_index: None,
                full_path: "/".into(),
            },
            parent_index: 0,
            // this value will need to be updated later on
            next_index: 0,
        });
        let mut rb_info = RebuildInfo {
            entries: vec![root_entry],
            // Later in this function, `file_offset` will be offset more
            // once the fst size is known (with the `extra` variable)
            file_offset: 0,
            filename_offset: 0,
            file_count: 0,
            parent_index: None,
            current_path: "/".into(),
        };

        FST::rebuild_dir_info(root_path.as_ref(), &mut rb_info)?;

        rb_info.entries[0].as_dir_mut().unwrap().next_index = rb_info.entries.len();

        let size = rb_info.entries.len() * 12 + rb_info.filename_offset as usize;
        let total_file_system_size = rb_info.file_offset as usize;

        let offset = align(APPLOADER_OFFSET + appldr_size as u64);
        let extra = offset + align(size as u64) + align(dol_size);

        for e in &mut rb_info.entries {
            if let Some(ref mut f) = e.as_file_mut() {
                f.file_offset += extra;
            }
        }

        Ok(FST {
            offset,
            file_count: rb_info.file_count,
            entries: rb_info.entries,
            total_file_system_size,
            size,
        })
    }

    // this needs to be documented, specifically how rb_info is being used
    // it's also a mess...
    fn rebuild_dir_info<P>(path: P, rb_info: &mut RebuildInfo) -> io::Result<()>
        where P: AsRef<Path>
    {
        for e in read_dir(path.as_ref())? {
            let e = e?;

            // TODO: don't keep calling e.file_name(), store it somewhere

            if e.file_name().to_str().map(|s| s.starts_with(".")).unwrap_or(false) ||
               e.file_name().to_str() == Some("&&systemdata") { continue }

            let mut full_path = rb_info.current_path.clone();
            full_path.push(e.file_name());
            let index = rb_info.entries.len() as usize;
            let info = EntryInfo {
                index,
                name: e.file_name().to_string_lossy().into_owned(),
                filename_offset: rb_info.filename_offset,
                directory_index: rb_info.parent_index,
                full_path,
            };
            // plus 1 for the null byte
            rb_info.filename_offset += info.name.chars().count() as u64 + 1;

            if e.file_type()?.is_dir() {
                let old_index = rb_info.parent_index;

                let entry = Entry::Directory(DirectoryEntry {
                    info,
                    parent_index: old_index.unwrap_or(0),
                    next_index: index + 1
                });
                let index = rb_info.entries.len();
                rb_info.entries.push(entry);

                rb_info.parent_index = Some(index);
                rb_info.current_path.push(e.file_name());
                let count_before = rb_info.entries.len();

                FST::rebuild_dir_info(e.path(), rb_info)?;

                rb_info.parent_index = old_index;
                rb_info.current_path.pop();
                rb_info.entries[index].as_dir_mut().unwrap().next_index += rb_info.entries.len() - count_before;
            } else {
                let entry = Entry::File(FileEntry {
                    info,
                    file_offset: rb_info.file_offset,
                    size: e.metadata()?.len() as usize,
                });
                rb_info.file_offset += align(entry.as_file().unwrap().size as u64);
                rb_info.file_count += 1;
                rb_info.entries.push(entry);
            }
        }
        Ok(())
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut sorted_names = BTreeMap::new();
        for e in &self.entries {
            e.write(writer)?;
            sorted_names.insert(e.info().filename_offset, &e.info().name);
        }
        let null_byte = [0];
        for (_, name) in &sorted_names {
            writer.write(name.as_bytes())?;
            writer.write(&null_byte[..])?;
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

        if name == Path::new(&entry.info().full_path.strip_prefix("/").unwrap()) {
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
        "&&systemdata/Game.toc".into()
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

    fn print_info(&self) {
        println!("Offset: {}", self.offset);
        println!("Total entries: {}", self.entries.len());
        println!("Total files: {}", self.file_count);
        println!("Total space used by files: {} bytes", self.total_file_system_size);
        println!("Size: {} bytes", self.size);
    }
}

impl<'a> UniqueLayoutSection<'a> for FST {
    fn section_type(&self) -> UniqueSectionType {
        UniqueSectionType::FST
    }

    fn with_offset(
        file: &mut BufReader<impl ReadSeek>,
        offset: u64,
    ) -> io::Result<FST> {
        FST::new(file, offset)
    }
}

