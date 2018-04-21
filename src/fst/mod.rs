pub mod entry;

use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::fs::{File, read_dir};
use std::collections::BTreeMap;

use byteorder::{BigEndian, ReadBytesExt};

use self::entry::{DirectoryEntry, Entry, EntryInfo, FileEntry, ENTRY_SIZE};
use app_loader::APPLOADER_OFFSET;
use ::{align, extract_section};

pub const FST_OFFSET_OFFSET: u64 = 0x0424; 
pub const FST_SIZE_OFFSET: u64 = 0x0428;

struct RebuildInfo {
    entries: Vec<Entry>,
    file_offset: u64,
    filename_offset: u64,
    file_count: usize,
    parent_index: Option<usize>,
}

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

        let size = (iso.seek(SeekFrom::Current(0))? - fst_offset) as usize;

        Ok(FST {
            file_count,
            total_file_system_size,
            entries,
            size,
        })
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
        };

        FST::rebuild_dir_info(root_path.as_ref(), &mut rb_info)?;

        rb_info.entries[0].as_dir_mut().unwrap().next_index = rb_info.entries.len();

        let size = rb_info.entries.len() * 12 + rb_info.filename_offset as usize;
        let total_file_system_size = rb_info.file_offset as usize;

        let extra =
            align(APPLOADER_OFFSET + appldr_size as u64) +
            align(size as u64) +
            align(dol_size);

        for e in &mut rb_info.entries {
            if let Some(ref mut f) = e.as_file_mut() {
                f.file_offset += extra;
            }
        }

        Ok(FST {
            file_count: rb_info.file_count,
            entries: rb_info.entries,
            total_file_system_size,
            size,
        })
    }

    // this needs to be documented, specifically how rb_info is being used
    // it's also a mess...
    fn rebuild_dir_info<P>(path: P, rb_info: &mut RebuildInfo) -> io::Result<usize>
        where P: AsRef<Path>
    {
        let mut total_child_count = 0;
        for e in read_dir(path.as_ref())? {
            let e = e?;

            if e.file_name().to_str().map(|s| s.starts_with(".")).unwrap_or(false) ||
               e.file_name().to_str() == Some("&&systemdata") { continue }

            let index = rb_info.entries.len() as usize;
            let info = EntryInfo {
                index,
                name: e.file_name().to_string_lossy().into_owned(),
                filename_offset: rb_info.filename_offset,
            };
            // plus 1 for the null byte
            rb_info.filename_offset += info.name.chars().count() as u64 + 1;

            if e.file_type()?.is_dir() {
                let old_index = rb_info.parent_index;
                // Use this once Option::filter stabilizes (it'll be soon):
                // let children_count = read_dir(e.path())?.filter_map(|e|
                    // e.as_ref().ok().and_then(|e|
                        // e.file_name().to_str().filter(|s| !s.starts_with("."))
                    // )
                // ).count();
                let children_count = read_dir(e.path())?.filter(|e|
                    e.as_ref().ok().and_then(|e|
                        e.file_name().to_str().map(|s| !s.starts_with("."))
                    ).unwrap_or(false)
                ).count();

                total_child_count += children_count;
                let entry = Entry::Directory(DirectoryEntry {
                    info,
                    parent_index: old_index.unwrap_or(0),
                    next_index: index + children_count + 1
                });
                let index = rb_info.entries.len();
                rb_info.entries.push(entry);
                rb_info.parent_index = Some(index);

                let sub_count = FST::rebuild_dir_info(e.path(), rb_info)?;
                rb_info.entries[index].as_dir_mut().unwrap().next_index += sub_count;
                rb_info.parent_index = old_index;
            } else {
                let entry = Entry::File(FileEntry {
                    info,
                    file_offset: rb_info.file_offset,
                    length: e.metadata()?.len() as usize,
                });
                rb_info.file_offset += align(entry.as_file().unwrap().length as u64);
                rb_info.file_count += 1;
                rb_info.entries.push(entry);
            }
        }
        Ok(total_child_count)
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
}

