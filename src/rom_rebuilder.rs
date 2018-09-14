use std::cmp;
use std::fs::{File, read_dir};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sections::apploader::APPLOADER_OFFSET;
use sections::fst::{
    FST,
    entry::{DirectoryEntry, Entry, EntryInfo, FileEntry},
};
use sections::header::Header;
use ::{
    align,
    DEFAULT_ALIGNMENT,
    extract_section,
    paths::*,
    WRITE_CHUNK_SIZE,
};

pub const ROM_SIZE: usize = 0x57058000;

// TODO: modify the config struct to include stuff like whether the system data should be rebuilt
// and the paths for stuff like the dol, apploader, fst, and so on...

// Header -> apploader -> fst -> dol -> fs

struct ROMConfig<'a> {
    alignment: u64,
    root: &'a Path,
    files: Vec<(u64, PathBuf)>,
}

struct FSTRebuilderInfo {
    entries: Vec<Entry>,
    file_offset: u64,
    filename_offset: u64,
    file_count: usize,
    parent_index: Option<usize>,
    current_path: PathBuf,
}

struct FSTRebuilder<'a> {
    apploader_size: usize,
    dol_size: usize,
    config: ROMConfig<'a>,
}

impl<'a> FSTRebuilder<'a> {
    fn new<P: ?Sized>(root: &'a P, alignment: u64) -> io::Result<FSTRebuilder<'a>>
    where
        P: AsRef<Path>,
    {
        let apploader = File::open(root.as_ref().join(APPLOADER_PATH))?;
        let apploader_size = apploader.metadata()?.len() as usize;

        let dol = File::open(root.as_ref().join(DOL_PATH))?;
        let dol_size = dol.metadata()?.len() as usize;

        Ok(FSTRebuilder {
            apploader_size,
            dol_size,
            config: ROMConfig {
                alignment,
                root: root.as_ref(),
                files: vec![],
            },
        })
    }

    fn rebuild(self) -> io::Result<HeaderRebuilder<'a>> {
        let root_entry = Entry::Directory(DirectoryEntry {
            info: EntryInfo {
                index: 0,
                name: "/".to_owned(),
                filename_offset: 0,
                directory_index: None,
                full_path: "/".into(),
            },
            parent_index: 0,
            // this value is updated later on, after self.rebuild_dir_info is called
            next_index: 0,
        });
        let mut rb_info = FSTRebuilderInfo {
            entries: vec![root_entry],
            // Later in this function, `file_offset` will be offset more
            // once the fst size is known (with the `file_system_offset` variable)
            file_offset: 0,
            filename_offset: 0,
            file_count: 0,
            parent_index: None,
            current_path: "/".into(),
        };

        self.rebuild_dir_info(self.config.root, &mut rb_info)?;

        rb_info.entries[0].as_dir_mut().unwrap().next_index =
            rb_info.entries.len();

        let size = rb_info.entries.len() * 12 + rb_info.filename_offset as usize;
        let offset = align(APPLOADER_OFFSET + self.apploader_size as u64, self.config.alignment);

        let dol_offset = offset + align(size as u64, self.config.alignment);
        let file_system_offset = dol_offset + align(self.dol_size as u64, self.config.alignment);

        for e in &mut rb_info.entries {
            if let Some(ref mut f) = e.as_file_mut() {
                f.file_offset += file_system_offset;
            }
        }

        let fst = FST {
            offset,
            file_count: rb_info.file_count,
            entries: rb_info.entries,
            total_file_system_size: rb_info.file_offset as usize,
            size,
        };
        let fst_path = self.config.root.join(FST_PATH);
        fst.write(File::create(&fst_path)?)?;

        Ok(HeaderRebuilder {
            dol_offset,
            fst,
            config: self.config,
        })
    }

    fn rebuild_dir_info(
        &self,
        path: impl AsRef<Path>,
        rb_info: &mut FSTRebuilderInfo,
    ) -> io::Result<()> {
        for e in read_dir(path.as_ref())? {
            let e = e?;

            // TODO: don't keep calling e.file_name(), store it somewhere

            if e.file_name().to_str().map(|s| s.starts_with("."))
                .unwrap_or(false)
                || e.file_name().to_str() == Some("&&systemdata")
            {
                continue
            }

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

                self.rebuild_dir_info(e.path(), rb_info)?;

                rb_info.parent_index = old_index;
                rb_info.current_path.pop();
                rb_info.entries[index].as_dir_mut().unwrap().next_index +=
                    rb_info.entries.len() - count_before;
            } else {
                // As noted in `rebuild`, this `file_offset` is not
                // the final offset. It'd be added to later.
                let entry = Entry::File(FileEntry {
                    info,
                    file_offset: rb_info.file_offset,
                    size: e.metadata()?.len() as usize,
                });
                rb_info.file_offset +=
                    align(entry.as_file().unwrap().size as u64, self.config.alignment);
                rb_info.file_count += 1;
                rb_info.entries.push(entry);
            }
        }
        Ok(())
    }
}

struct HeaderRebuilder<'a> {
    dol_offset: u64,
    fst: FST,
    config: ROMConfig<'a>,
}

impl<'a> HeaderRebuilder<'a> {
   fn rebuild(self) -> io::Result<FileSystemRebuilder<'a>> {
        let header_path = self.config.root.join(HEADER_PATH);
        let header_buf = BufReader::new(File::open(&header_path)?);
        let mut header = Header::new(header_buf, 0)?;

        header.dol_offset = self.dol_offset as u64;
        header.fst_offset = self.fst.offset as u64;
        header.fst_size = self.fst.size;

        // TODO: Is this okay to assume?
        header.max_fst_size = self.fst.size;

        header.write(File::create(&header_path)?)?;

        Ok(FileSystemRebuilder {
            fst: self.fst,
            header,
            config: self.config,
        })
    }
}

struct FileSystemRebuilder<'a> {
    fst: FST,
    header: Header,
    config: ROMConfig<'a>,
}

impl<'a> FileSystemRebuilder<'a> {
    fn rebuild(mut self) -> io::Result<ROMRebuilder> {
        let apploader_path = self.config.root.join(APPLOADER_PATH);
        let dol_path = self.config.root.join(DOL_PATH);
        let fst_path = self.config.root.join(FST_PATH);
        let header_path = self.config.root.join(HEADER_PATH);

        self.config.files.push((APPLOADER_OFFSET, apploader_path));
        self.config.files.push((self.header.dol_offset, dol_path));
        self.config.files.push((self.fst.offset, fst_path));
        self.config.files.push((0, header_path));

        FileSystemRebuilder::fill_files(&mut self.config.files, self.fst.entries[0].as_dir().unwrap(), self.config.root, &self.fst);

        self.config.files.sort();

        Ok(ROMRebuilder {
            files: self.config.files,
        })
    }

    fn fill_files(
        files: &mut Vec<(u64, PathBuf)>,
        dir: &DirectoryEntry,
        prefix: impl AsRef<Path>,
        fst: &FST,
    ) {
        for entry in dir.iter_contents(&fst.entries) {
            match entry {
                Entry::File(ref file) => {
                    files.push((
                        file.file_offset,
                        prefix.as_ref().join(&file.info.name),
                    ));
                },
                Entry::Directory(ref sub_dir) => {
                    FileSystemRebuilder::fill_files(
                        files,
                        sub_dir,
                        prefix.as_ref().join(&sub_dir.info.name),
                        fst,
                    );
                },
            };
        }
    }
}

pub struct ROMRebuilder {
    files: Vec<(u64, PathBuf)>,
}

impl ROMRebuilder {
    pub fn rebuild(root: impl AsRef<Path>, alignment: u64, output: impl Write, rebuild_systemdata: bool) -> io::Result<()> {
        if rebuild_systemdata {
            FSTRebuilder::new(root.as_ref(), alignment)?
                .rebuild()?
                .rebuild()?
                .rebuild()?
                .write(output)
        } else {
            let fst_file = File::open(root.as_ref().join(FST_PATH))?;
            let header_file = File::open(root.as_ref().join(HEADER_PATH))?;
            let fst = FST::new(BufReader::new(fst_file), 0)?;
            let header = Header::new(BufReader::new(header_file), 0)?;
            FileSystemRebuilder {
                fst,
                header,
                config: ROMConfig {
                    alignment,
                    root: root.as_ref(),
                    files: vec![],
                }
            }.rebuild()?.write(output)
        }
    }

    fn write(
        &self,
        mut output: impl Write,
    ) -> io::Result<()> {
        let mut bytes_written = 0;
        let total_files = self.files.len();

        for (i, &(offset, ref filename)) in self.files.iter().enumerate() {
            let mut file = File::open(filename)?;
            let size = file.metadata()?.len();
            if size == 0 { continue }

            write_zeros((offset - bytes_written) as usize, &mut output)?;
            bytes_written = offset;

            extract_section(&mut file, size as usize, &mut output)?;
            bytes_written += size;

            if bytes_written as usize > ROM_SIZE {
                println!();
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Error: not enough space. Try decreasing the file alignment with the -a option (the default is {} bytes).",
                        DEFAULT_ALIGNMENT,
                    ),
                ));
            }
            print!("\r{}/{} files added.", i + 1, total_files);
        }
        println!();
        write_zeros(ROM_SIZE - bytes_written as usize, &mut output)
    }
}

fn write_zeros(count: usize, mut output: impl Write) -> io::Result<()> {
    lazy_static! {
        static ref ZEROS: Mutex<Vec<u8>> = Mutex::new(vec![]);
    }
    let mut zeros = ZEROS.lock().unwrap();
    let block_size = cmp::min(count, WRITE_CHUNK_SIZE);
    zeros.resize(block_size, 0);
    for i in 0..(count / WRITE_CHUNK_SIZE + 1) {
        (&mut output).write_all(
            &zeros[..cmp::min(WRITE_CHUNK_SIZE, count - WRITE_CHUNK_SIZE * i)]
        )?;
    }
    Ok(())
}
