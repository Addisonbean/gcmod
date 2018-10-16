extern crate clap;
extern crate gcmod;
extern crate tempfile;

use std::env;
use std::fs::{remove_file, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use clap::{App, Arg, SubCommand, AppSettings};

use gcmod::{
    AppError,
    AppResult,
    DEFAULT_ALIGNMENT,
    Game,
    format_u64,
    format_usize,
    MIN_ALIGNMENT,
    NumberStyle,
    parse_as_u64,
    ROM_SIZE,
};
use gcmod::{Disassembler, ROMRebuilder};
use gcmod::sections::{
    dol::DOLHeader,
    layout_section::{LayoutSection, UniqueSectionType}
};

fn main() -> AppResult {
    let opts = App::new("gciso")

        .subcommand(SubCommand::with_name("extract")
            .about("Extract a ROM's contents to disk.")
            .arg(Arg::with_name("rom_path").required(true))
            .arg(Arg::with_name("output").required(true))
            .arg(Arg::with_name("rom_section").short("s").long("section")
                .takes_value(true).required(false)
                .help("Specify a single section to extract from the ROM, rather than everything.")))

        .subcommand(SubCommand::with_name("info")
            .about("Display information about the ROM.")
            .arg(Arg::with_name("hex_output").short("h").long("hex")
                 .required(false)
                 .help("Displays numbers in hexadecimal."))
            .arg(Arg::with_name("rom_path").required(true))
            .arg(Arg::with_name("type").short("t").long("type")
                 .takes_value(true).required(false)
                 .possible_values(&[
                    "header",
                    "dol",
                    "fst",
                    "apploader",
                    "layout",
                 ])
                 .case_insensitive(true)
                 .help("Print a given type of information about the ROM."))
            .arg(Arg::with_name("offset").short("o").long("offset")
                 .takes_value(true).required(false)
                 .conflicts_with_all(&["type", "mem_addr"])
                 .help("Print information about whichever section is at the given offset."))
            .arg(Arg::with_name("mem_addr").short("m").long("mem_addr")
                 .takes_value(true).required(false)
                 .conflicts_with_all(&["type", "offset"])
                 .help("Print information about the DOL segment that will be loaded into a given address in memory.")))

        // TODO: add flags for searching and crap
        // Add more `ls` style flags (LS_COLORS!)
        // Add a flag to recursively list, default to / or the dir they pass
        .subcommand(SubCommand::with_name("ls")
            .about("Lists the files on the ROM.")
            .arg(Arg::with_name("rom_path").required(true))
            .arg(Arg::with_name("dir").required(false)
                 .help("The name or path of the directory to list."))
            .arg(Arg::with_name("long").short("l").long("long")
                 .required(false)))

        .subcommand(SubCommand::with_name("disasm")
            .about("Disassemble the main DOL file from a ROM.")
            .arg(Arg::with_name("rom_path").required(true))
            .arg(Arg::with_name("objdump_path").long("objdump")
                 .takes_value(true).required(false)
                 .help("If you don't have the GNU version of objdump in $PATH, you must either provide the path here or set it in the $GCMOD_GNU_OBJDUMP enviroment variable.")))

        .subcommand(SubCommand::with_name("rebuild")
            .about("Rebuilds a ROM.")
            .arg(Arg::with_name("root_path").required(true))
            .arg(Arg::with_name("output").required(true))
            .arg(Arg::with_name("alignment").short("a").long("alignment")
                .required(false)
                .takes_value(true)
                .help("Specifies the alignment in bytes for the files in the filesystem. The default is 32768 bytes (32KiB) and the minimum is 2 bytes.")))

        .setting(AppSettings::SubcommandRequired);
    match opts.get_matches().subcommand() {
        ("extract", Some(cmd)) => 
            extract_iso(
                cmd.value_of("rom_path").unwrap(),
                cmd.value_of("output").unwrap(),
                cmd.value_of("rom_section"),
            ),
        ("info", Some(cmd)) => 
            get_info(
                cmd.value_of("rom_path").unwrap(),
                cmd.value_of("type"),
                cmd.value_of("offset"),
                cmd.value_of("mem_addr"),
                if cmd.is_present("hex_output") {
                    NumberStyle::Hexadecimal
                } else {
                    NumberStyle::Decimal
                },
            ),
        ("ls", Some(cmd)) =>
            ls_files(
                cmd.value_of("rom_path").unwrap(),
                cmd.value_of("dir"),
                cmd.is_present("long"),
            ),
        ("disasm", Some(cmd)) =>
            disassemble_dol(
                cmd.value_of("rom_path").unwrap(),
                cmd.value_of("objdump_path"),
            ),
        ("rebuild", Some(cmd)) =>
            rebuild_iso(
                cmd.value_of("root_path").unwrap(),
                cmd.value_of("output").unwrap(),
                cmd.value_of("alignment"),
                true,
            ),
        _ => unreachable!(),
    }
}

fn extract_iso(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    file_in_iso: Option<impl AsRef<Path>>,
) -> AppResult {
    let output = output.as_ref();

    if let Some(file) = file_in_iso {
        return extract_section(input.as_ref(), file.as_ref(), output);
    }

    if output.exists() {
        return Err(AppError::new(format!("Error: {} already exists.", output.display())));
    }

    let (mut game, mut iso) = try_to_open_game(input.as_ref(), 0)?;
    game.extract(&mut iso, output).map_err(|_| AppError::new("Failed to write files."))?;

    Ok(())
}

fn print_iso_info(input: impl AsRef<Path>, offset: u64, style: NumberStyle) -> AppResult {
    let (game, _) = try_to_open_game(input, offset)?;
    game.print_info(style);
    Ok(())
}

// is this a bit much for main.rs? Move it to disassembler.rs?
fn disassemble_dol(
    input: impl AsRef<Path>,
    objdump_path: Option<impl AsRef<Path>>
) -> AppResult {
    let (game, mut iso) = try_to_open_game(input.as_ref(), 0)?;

    let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
    DOLHeader::extract(&mut iso, tmp_file.as_mut(), game.dol.offset)
        .map_err(|_| AppError::new("Could not extract dol."))?;

    tmp_file.seek(SeekFrom::Start(0)).unwrap();
    let header = DOLHeader::new(tmp_file.as_mut(), 0)
        .expect("Failed to read header.");

    let objdump_path = objdump_path
        .map(|p| p.as_ref().to_path_buf())
        .or_else(|| env::var("GCMOD_GNU_OBJDUMP").ok().map(|p| PathBuf::from(p)))
        .unwrap_or_else(|| PathBuf::from("objdump"));

    let disassembler = Disassembler::objdump_path(objdump_path.as_os_str())
        .map_err(|_| AppError::new("GNU objdump required."))?;

    let mut addr = 0;
    for s in header.iter_segments() {
        if s.size == 0 { continue }
        println!("{}", s.to_string());

        let disasm = disassembler.disasm(tmp_file.path(), s)
            .expect("Failed to open DOL section");
        for instr in disasm {
            addr = instr.location.unwrap_or(addr + 4);
            println!(
                "{:#010x}: {:#010x} {}",
                addr, instr.opcode, instr.text,
            );
            if instr.location.is_none() {
                println!("                       ...");
            }
        }
    }
    Ok(())
}

fn rebuild_iso(
    root_path: impl AsRef<Path>,
    iso_path: impl AsRef<Path>,
    alignment: Option<&str>,
    rebuild_systemdata: bool,
) -> AppResult {
    let alignment = match alignment {
        Some(a) => match parse_as_u64(a) {
            Ok(a) if a >= MIN_ALIGNMENT => a,
            _ => return Err(AppError::new(format!("Invalid alignment. Must be an integer >= {}", MIN_ALIGNMENT))),
        },
        None => DEFAULT_ALIGNMENT,
    };

    let iso_path = iso_path.as_ref();

    if iso_path.exists() {
        return Err(AppError::new(format!("{} already exists.", iso_path.display())));
    }

    let iso = File::create(iso_path).unwrap(); 
    if let Err(_) = ROMRebuilder::rebuild(root_path, alignment, iso, rebuild_systemdata) {
        remove_file(iso_path).unwrap();
        Err(AppError::new("Couldn't rebuild iso."))
    } else {
        Ok(())
    }
}

fn get_info(
    path: impl AsRef<Path>,
    section: Option<&str>,
    offset: Option<&str>,
    mem_addr: Option<&str>,
    style: NumberStyle,
) -> AppResult {
    use gcmod::sections::layout_section::UniqueSectionType::*;

    if let Some(offset) = offset {
        find_offset(path.as_ref(), offset, style)
    } else if let Some(addr) = mem_addr {
        find_mem_addr(path.as_ref(), addr, style)
    } else {
        match section {
            Some("header") => print_section_info(path.as_ref(), &Header, style),
            Some("dol") => print_section_info(path.as_ref(), &DOLHeader, style),
            Some("fst") => print_section_info(path.as_ref(), &FST, style),
            Some("apploader") | Some("app_loader") | Some("app-loader") =>
                print_section_info(path.as_ref(), &Apploader, style),
            Some("layout") => print_layout(path.as_ref()),
            Some(_) => unreachable!(),
            None => print_iso_info(path.as_ref(), 0, style),
        }
    }
}

fn print_section_info(
    path: impl AsRef<Path>,
    section_type: &UniqueSectionType,
    style: NumberStyle,
) -> AppResult {
    let mut f = File::open(path.as_ref())
        .map(BufReader::new)
        .map_err(|_| AppError::new("Couldn't open file"))?;

    if let Ok(g) = Game::open(&mut f, 0) {
        g.get_section_by_type(section_type).print_info(style);
    } else if let Ok(s) = section_type.with_offset(&mut f, 0) {
        s.print_info(style);
    } else {
        return Err(AppError::new("Invalid file"))
    }

    Ok(())
}

fn print_layout(path: impl AsRef<Path>) -> AppResult {
    let (game, _) = try_to_open_game(path.as_ref(), 0)?;
    game.print_layout();
    Ok(())
}

fn find_offset(header_path: impl AsRef<Path>, offset: &str, style: NumberStyle) -> AppResult {
    let offset = parse_as_u64(offset).ok()
        .filter(|o| (*o as usize) < ROM_SIZE)
        .ok_or_else(|| AppError::new(format!(
            "Invalid offset. Offset must be a number > 0 and < {}",
            format_usize(ROM_SIZE, style),
        )))?;

    let (game, _) = try_to_open_game(header_path.as_ref(), 0)?;
    let layout = game.rom_layout();
    let section = layout.find_offset(offset)
        .ok_or_else(|| AppError::new("There isn't any data at this offset."))?;

    section.print_info(style);
    Ok(())
}

fn find_mem_addr(path: impl AsRef<Path>, mem_addr: &str, style: NumberStyle) -> AppResult {
    let mem_addr = parse_as_u64(mem_addr)
        .map_err(|_| AppError::new("Invalid address. Must be an integer."))?;

    let (game, _) = try_to_open_game(path.as_ref(), 0)?;

    let seg = game.dol.segment_at_addr(mem_addr)
        .ok_or_else(|| AppError::new("No DOL segment will be loaded at this address."))?;

    let offset = mem_addr - seg.loading_address;
    println!("Segment: {}", seg.name());
    println!("Offset from start of segment: {}", format_u64(offset, style));

    Ok(())
}

fn extract_section(
    iso_path: impl AsRef<Path>,
    section_filename: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> AppResult {
    let (game, mut iso) = try_to_open_game(iso_path.as_ref(), 0)?;

    let result = game.extract_section_with_name(
        section_filename,
        output.as_ref(),
        &mut iso,
    );

    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(AppError::new("Couldn't find a section with that name.")),
        Err(_) => Err(AppError::new("Error extracting section.")),
    }
}

fn ls_files(rom_path: impl AsRef<Path>, dir: Option<impl AsRef<Path>>, long_format: bool) -> AppResult {
    let (game, _) = try_to_open_game(rom_path, 0)?;
    let dir = match dir {
        Some(p) => game.fst.entry_for_path(p).and_then(|e| e.as_dir()),
        None => Some(game.fst.root()),
    };

    if let Some(d) = dir {
        game.print_directory(d, long_format);
        Ok(())
    } else {
        Err(AppError::new("No directory with that name/path exists"))
    }
}

fn try_to_open_game<P>(path: P, offset: u64) -> Result<(Game, BufReader<File>), AppError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if !path.exists() {
        return Err(AppError::new(format!("The iso {} doesn't exist.", path.display())));
    }

    let iso = File::open(path).expect("Couldn't open file");
    let mut iso = BufReader::new(iso);
    Game::open(&mut iso, offset)
        .map(|game| (game, iso))
        .map_err(|_| AppError::new(format!("Invalid iso: {}.", path.display())))
}
