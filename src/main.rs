extern crate gamecube_iso_assistant;
extern crate clap;
extern crate tempfile;

use std::path::Path;
use std::io::{Seek, SeekFrom};
use std::fs::File;

use clap::{App, Arg, SubCommand, AppSettings};

use gamecube_iso_assistant::Game;
use gamecube_iso_assistant::dol::DOLHeader;
use gamecube_iso_assistant::disassembler::Disassembler;

fn main() {
    let opts = App::new("gciso")

        .subcommand(SubCommand::with_name("extract")
            .about("Extract a ROM's contents to disk.")
            .arg(Arg::with_name("path_to_iso").short("i").long("iso")
                 .takes_value(true).required(true))
            .arg(Arg::with_name("output_dir").short("o").long("output")
                 .takes_value(true).required(true)))

        .subcommand(SubCommand::with_name("info")
            .about("Display information about the ROM.")
            .arg(Arg::with_name("path_to_iso").short("i").long("iso")
                 .takes_value(true).required(true)))

        .subcommand(SubCommand::with_name("disasm")
            .about("Disassemble the main DOL file from an iso.")
            .arg(Arg::with_name("path_to_iso").short("-i").long("iso")
                 .takes_value(true).required(true)))

        .subcommand(SubCommand::with_name("rebuild")
            .about("Rebuilds an iso.")
            .arg(Arg::with_name("path_to_iso").short("-i").long("iso")
                 .takes_value(true).required(true))
            .arg(Arg::with_name("path_to_root").short("-r").long("root")
                 .takes_value(true).required(true)))

        .setting(AppSettings::SubcommandRequired);
    match opts.get_matches().subcommand() {
        ("extract", Some(cmd)) => 
            extract_iso(
                cmd.value_of("path_to_iso").unwrap(),
                cmd.value_of("output_dir").unwrap()
            ),
        ("info", Some(cmd)) => 
            print_iso_info(cmd.value_of("path_to_iso").unwrap()),
        ("disasm", Some(cmd)) =>
            disassemble_dol(cmd.value_of("path_to_iso").unwrap()),
        ("rebuild", Some(cmd)) =>
            rebuild_iso(
                cmd.value_of("path_to_root").unwrap(),
                cmd.value_of("path_to_iso").unwrap()
            ),
        _ => (),
    }
}

fn extract_iso<P: AsRef<Path>>(input: P, output: P) {
    let output = output.as_ref();
    if output.exists() {
        eprintln!("Error: {} already exists.", output.display());
    } else {
        try_to_open_game(input.as_ref()).map(|mut game|
            if let Err(_) = game.extract(output) {
                eprintln!("Failed to write files.");
            }
        );
    }
}

fn print_iso_info<P: AsRef<Path>>(input: P) {
    try_to_open_game(input).as_ref().map(Game::print_info);
}

fn disassemble_dol<P: AsRef<Path>>(input: P) {
    try_to_open_game(input.as_ref()).map(|mut game| {
        let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
        if let Err(_) = game.write_dol(tmp_file.as_mut()) {
            eprintln!("Could not extract dol.");
        }
        tmp_file.seek(SeekFrom::Start(0)).unwrap();
        let header = DOLHeader::new(tmp_file.as_mut())
            .expect("Failed to read header.");
        let disassembler = match Disassembler::objdump_path(&"/usr/local/Cellar/binutils/2.29/bin/gobjdump") {
            Ok(d) => d,
            Err(_) => {
                eprintln!("GNU objdump required.");
                return;
            },
        };

        // TODO: remove the redundancy here
        let mut addr = 0;
        for (i, s) in header.text_segments.iter().enumerate() {
            if s.size == 0 { continue }
            println!("{}", s.seg_type.to_string(i));

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

        for (i, s) in header.data_segments.iter().enumerate() {
            if s.size == 0 { continue }
            println!("{}", s.seg_type.to_string(i));

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

// TODO: don't unwrap/expect
fn rebuild_iso<P: AsRef<Path>>(root_path: P, iso_path: P) {
    let mut iso = File::create(iso_path.as_ref()).unwrap(); 
    Game::rebuild(root_path.as_ref(), &mut iso).expect("Couldn't rebuild iso.");
}

fn try_to_open_game<P: AsRef<Path>>(path: P) -> Option<Game> {
    let path = path.as_ref();
    if !path.exists() {
        eprintln!("Error: the iso {} doesn't exist.", path.display());
    } else {
        match Game::open(path) {
            Ok(game) => return Some(game),
            Err(_) => eprintln!("Invalid iso: {}.", path.display()),
        }
    }
    None
}

