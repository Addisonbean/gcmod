#[macro_use]
extern crate clap;
extern crate gcmod;
extern crate tempfile;

use std::fs::{remove_file, File};
use std::io::BufReader;
use std::path::Path;

use clap::AppSettings;

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
    sections::{
        apploader::Apploader,
        dol::DOLHeader,
        fst::FST,
        header::Header,
        Section,
    },
};
use gcmod::ROMRebuilder;

fn main() -> AppResult {
    let app = clap_app!(app =>
        (@subcommand extract =>
            (about: "Extract a ROM's contents to disk.")
            (@arg rom_path: +required)
            (@arg output: +required)
            (@arg rom_section: -s --section +takes_value "Specify a single section to extract from the ROM, rather than everything.")
        )
        (@subcommand info =>
            (about: "Display information about the ROM.")
            (@arg rom_path: +required)
            (@arg hex_output: -h --hex "Displays numbers in hexadecimal.")
            (@arg type: -t --type +takes_value +case_insensitive
                possible_value[header dol fst apploader layout]
                "Print a given type of information about the ROM.")
            (@arg offset: -o --offset +takes_value
                conflicts_with[type mem_addr]
                "Print information about whichever section is at the given offset.")
            (@arg mem_addr: -m --("mem-addr") +takes_value
                conflicts_with[type offset]
                "Print information about the DOL segment that will be loaded into a given address in memory.")
        )
        // TODO: add flags for searching and crap
        // Add more `ls` style flags (LS_COLORS!)
        // Add a flag to recursively list, default to / or the dir they pass
        (@subcommand ls =>
            (about: "Lists the files on the ROM.")
            (@arg rom_path: +required)
            (@arg dir: "The name or path of the directory in the ROM to list.")
            (@arg long: -l --long "List the files in an `ls -l`-style format.")
        )
        (@subcommand rebuild =>
            (about: "Rebuilds a ROM.")
            (@arg root_path: +required)
            (@arg output: +required)
            (@arg no_rebuild_fst: --("no-rebuild-fst") "It this flag is passed, the existing file system table will be used, rather than creating a new one.")
            (@arg alignment: -a --alignment +takes_value
                "Specifies the alignment in bytes for the files in the filesystem. The default is 32768 bytes (32KiB) and the minimum is 2 bytes.")
        )
    ).setting(AppSettings::SubcommandRequired);

    match app.get_matches().subcommand() {
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
        ("rebuild", Some(cmd)) =>
            rebuild_iso(
                cmd.value_of("root_path").unwrap(),
                cmd.value_of("output").unwrap(),
                cmd.value_of("alignment"),
                !cmd.is_present("no_rebuild_fst"),
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
    game.extract(&mut iso, output).map_err(|_| AppError::new("Failed to write files."))
}

fn print_iso_info(input: impl AsRef<Path>, offset: u64, style: NumberStyle) -> AppResult {
    let (game, _) = try_to_open_game(input, offset)?;
    game.print_info(style);
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
    let root_path = root_path.as_ref();

    if iso_path.exists() {
        return Err(AppError::new(format!("{} already exists.", iso_path.display())));
    }
    if !root_path.exists() {
        return Err(AppError::new("Couldn't find root."));
    }

    let iso = File::create(iso_path)?;
    if let Err(_) = ROMRebuilder::rebuild(root_path, alignment, iso, rebuild_systemdata) {
        remove_file(iso_path).unwrap();
        Err(AppError::new("Couldn't rebuild iso."))
    } else {
        Ok(())
    }
}

fn get_info(
    path: impl AsRef<Path>,
    section_type: Option<&str>,
    offset: Option<&str>,
    mem_addr: Option<&str>,
    style: NumberStyle,
) -> AppResult {
    if let Some(offset) = offset {
        find_offset(path.as_ref(), offset, style)
    } else if let Some(addr) = mem_addr {
        find_mem_addr(path.as_ref(), addr, style)
    } else {
        let mut f = File::open(path.as_ref())
            .map(BufReader::new)
            .map_err(|_| AppError::new("Couldn't open file"))?;
        let game = Game::open(&mut f, 0);
        match section_type {
            Some("header") => {
                game
                    .map(|g| g.header)
                    .or_else(|_| Header::new(f, 0))
                    .map_err(|_| AppError::new("Invalid iso or header"))?
                    .print_info(style);
            },
            Some("dol") => {
                game
                    .map(|g| g.dol)
                    .or_else(|_| DOLHeader::new(f, 0))
                    .map_err(|_| AppError::new("Invalid iso or DOL"))?
                    .print_info(style);
            },
            Some("fst") => {
                game
                    .map(|g| g.fst)
                    .or_else(|_| FST::new(f, 0))
                    .map_err(|_| AppError::new("Invalid iso or file system table"))?
                    .print_info(style);
            },
            Some("apploader") | Some("app_loader") | Some("app-loader") => {
                game
                    .map(|g| g.apploader)
                    .or_else(|_| Apploader::new(f, 0))
                    .map_err(|_| AppError::new("Invalid iso or apploader"))?
                    .print_info(style);
            },
            Some("layout") => { print_layout(path.as_ref())?; }
            Some(_) => unreachable!(),
            None => { print_iso_info(path.as_ref(), 0, style)? },
        }
        Ok(())
    }
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
    println!("Segment: {}", seg.to_string());
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
