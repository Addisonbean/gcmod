extern crate clap;
extern crate gcmod;
extern crate tempfile;

use std::fs::{remove_file, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::mem::drop;
use std::path::{Path, PathBuf};

use clap::{App, Arg, SubCommand, AppSettings};

use gcmod::{
    DEFAULT_ALIGNMENT,
    Game,
    format_usize,
    MIN_ALIGNMENT,
    NumberStyle,
    parse_as_u64,
    ROM_SIZE,
};
use gcmod::disassembler::Disassembler;
use gcmod::dol::DOLHeader;
use gcmod::layout_section::UniqueSectionType;

fn main() {
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
            .arg(Arg::with_name("section").short("s").long("section")
                 .takes_value(true).required(false)
                 .possible_values(&[
                    "header",
                    "dol",
                    "fst",
                    "apploader",
                 ])
                 .case_insensitive(true)
                 .help("Print information about a given section of the ROM. "))
            .arg(Arg::with_name("offset").short("o").long("offset")
                 .takes_value(true).required(false)
                 .conflicts_with("section")
                 .help("Print information about whichever section is at the given offset. ")))

        .subcommand(SubCommand::with_name("disasm")
            .about("Disassemble the main DOL file from a ROM.")
            .arg(Arg::with_name("rom_path").required(true))
            .arg(Arg::with_name("objdump_path").long("objdump")
                 .takes_value(true).required(false)
                 .help("If you don't have the GNU version of objdump in $PATH, you must provide the path here.")))
                 // .help("If you don't have the GNU version of objdump in $PATH, you must either provide the path here or set it in the $GCMOD_OBJDUMP enviroment variable.")))

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
                cmd.value_of("section"),
                cmd.value_of("offset"),
                should_use_hex(cmd.is_present("hex_output")),
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
        _ => (),
    }
}

fn should_use_hex(b: bool) -> NumberStyle {
    if b { NumberStyle::Hexadecimal } else { NumberStyle::Decimal }
}

fn extract_iso(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    file_in_iso: Option<impl AsRef<Path>>,
) {
    if let Some(file) = file_in_iso {
        extract_section(input.as_ref(), file.as_ref(), output.as_ref());
        return;
    }

    let output = output.as_ref();
    if output.exists() {
        eprintln!("Error: {} already exists.", output.display());
    } else {
        try_to_open_game(input.as_ref(), 0).map(|(mut game, mut iso)|
            if let Err(_) = game.extract(&mut iso, output) {
                eprintln!("Failed to write files.");
            }
        );
    }
}

fn print_iso_info(input: impl AsRef<Path>, offset: u64, style: NumberStyle) {
    try_to_open_game(input, offset).map(|(game, _)| game.print_info(style));
}

// is this a bit much for main.rs? Move it to disassembler.rs?
fn disassemble_dol(
    input: impl AsRef<Path>,
    objdump_path: Option<impl AsRef<Path>>
) {
    try_to_open_game(input.as_ref(), 0).map(|(mut game, mut iso)| {
        let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
        if let Err(_) = game.extract_dol(&mut iso, tmp_file.as_mut()) {
            eprintln!("Could not extract dol.");
        }
        tmp_file.seek(SeekFrom::Start(0)).unwrap();
        let header = DOLHeader::new(tmp_file.as_mut(), 0)
            .expect("Failed to read header.");
        let objdump_path = objdump_path
            .map_or(PathBuf::from("objdump"), |p| p.as_ref().to_path_buf());
        let disassembler =
            match Disassembler::objdump_path(objdump_path.as_os_str()) {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("GNU objdump required.");
                    return;
                },
            };

        // TODO: remove the redundancy here
        let mut addr = 0;
        for s in header.text_segments.iter() {
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

        for s in header.data_segments.iter() {
            if s.size == 0 { continue }
            println!("{}", s.to_string());

            let disasm = disassembler.disasm(tmp_file.path(), s)
                .expect("Failed to open DOL section");
            for instr in disasm {
                addr = instr.location.unwrap_or(addr + 4);
                println!(
                    "{:#010x}: {:#010x} {}",
                    addr, instr.opcode, instr.text
                );
                if instr.location.is_none() {
                    println!("                       ...");
                }
            }
        }
    });
}

fn rebuild_iso(
    root_path: impl AsRef<Path>,
    iso_path: impl AsRef<Path>,
    alignment: Option<&str>,
    rebuild_systemdata: bool,
) {
    let alignment = match alignment {
        Some(a) => match parse_as_u64(a) {
            Ok(a) if a >= MIN_ALIGNMENT => a,
            _ => {
                eprintln!(
                    "Invalid alignment. Must be an integer >= {}",
                    MIN_ALIGNMENT,
                );
                return;
            },
        },
        None => DEFAULT_ALIGNMENT,
    };

    let mut iso = File::create(iso_path.as_ref()).unwrap(); 
    if let Err(e) =
        Game::rebuild(root_path.as_ref(), &mut iso, alignment, rebuild_systemdata)
    {
        eprintln!("Couldn't rebuild iso.");
        e.get_ref().map(|e| eprintln!("{}", e));
        drop(iso);
        remove_file(iso_path.as_ref()).unwrap();
    }
}

fn get_info(
    path: impl AsRef<Path>,
    section: Option<&str>,
    offset: Option<&str>,
    style: NumberStyle,
) {
    use gcmod::layout_section::UniqueSectionType::*;

    if let Some(offset) = offset {
        find_offset(path.as_ref(), offset, style);
    } else {
        match section {
            Some("header") => print_section_info(path.as_ref(), &Header, style),
            Some("dol") => print_section_info(path.as_ref(), &DOLHeader, style),
            Some("fst") => print_section_info(path.as_ref(), &FST, style),
            Some("apploader") | Some("app_loader") | Some("app-loader") =>
                print_section_info(path.as_ref(), &Apploader, style),
            Some(_) => unreachable!(),
            None => print_iso_info(path.as_ref(), 0, style),
        }
    }
}

fn print_section_info(
    path: impl AsRef<Path>,
    section_type: &UniqueSectionType,
    style: NumberStyle,
) {
    let mut f = match File::open(path.as_ref()) {
        Ok(f) => BufReader::new(f),
        Err(_) => {
            eprintln!("Couldn't open file");
            return;
        },
    };

    match Game::open(&mut f, 0) {
        Ok(g) => {
            g.get_section_by_type(section_type).print_info(style);
            return;
        },
        _ => (),
    }

    match section_type.with_offset(&mut f, 0) {
        Ok(s) => {
            s.print_info(style);
            return;
        },
        _ => (),
    }

    eprintln!("Invalid file");
}

fn find_offset(header_path: impl AsRef<Path>, offset: &str, style: NumberStyle) {
    let offset = match parse_as_u64(offset) {
        Ok(o) if (o as usize) < ROM_SIZE => o,
        _ => {
            eprintln!(
                "Invalid offset. Offset must be a number > 0 and < {}",
                format_usize(ROM_SIZE, style),
            );
            return;
        },
    };
    try_to_open_game(header_path.as_ref(), 0).map(|(game, _)| {
        // TODO: if None, tell if there's no data beyond this point
        // Also provide a message saying it's just blank space if it's None
        let layout = game.rom_layout();
        let section = match layout.find_offset(offset) {
            Some(s) => s,
            None => {
                eprintln!("There isn't any data at this offset.");
                return;
            }
        };

        section.print_section_info(style);
    });
}

fn extract_section(
    iso_path: impl AsRef<Path>,
    section_filename: impl AsRef<Path>,
    output: impl AsRef<Path>,
) {
    try_to_open_game(iso_path.as_ref(), 0).map(|(game, mut iso)| {
        let result = game.extract_section_with_name(
            section_filename,
            output.as_ref(),
            &mut iso,
        );
        match result {
            Ok(true) => {},
            Ok(false) => eprintln!("Couldn't find a section with that name."),
            Err(_) => eprintln!("Error extracting section."),
        }
    });
}

fn try_to_open_game<P>(path: P, offset: u64) -> Option<(Game, BufReader<File>)>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if !path.exists() {
        eprintln!("Error: the iso {} doesn't exist.", path.display());
    } else {
        let iso = File::open(path).expect("Couldn't open file");
        let mut iso = BufReader::new(iso);
        match Game::open(&mut iso, offset) {
            Ok(game) => return Some((game, iso)),
            Err(_) => eprintln!("Invalid iso: {}.", path.display()),
        }
    }
    None
}

