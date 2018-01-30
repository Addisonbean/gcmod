extern crate gamecube_iso_assistant;
extern crate rand;
extern crate clap;

use std::path::Path;

use clap::{App, Arg, SubCommand, AppSettings};

use gamecube_iso_assistant::Game;

fn main() {
    let opts = App::new("gciso")
        .subcommand(SubCommand::with_name("extract")
            .about("Extracts a ROM's contents to disk.")
            .arg(Arg::with_name("path_to_iso").short("i").long("iso")
                 .takes_value(true).required(true))
            .arg(Arg::with_name("output_dir").short("o").long("output")
                 .takes_value(true).required(true)))
        .subcommand(SubCommand::with_name("info")
            .about("Displays information about the ROM.")
            .arg(Arg::with_name("path_to_iso").short("i").long("iso")
                 .takes_value(true).required(true)))
        .setting(AppSettings::SubcommandRequired);
    match opts.get_matches().subcommand() {
        ("extract", Some(cmd)) => {
            let output_path = Path::new(cmd.value_of("output_dir").unwrap());
            let iso_path = Path::new(cmd.value_of("path_to_iso").unwrap());
            if !iso_path.exists() {
                eprintln!("Error: the iso {} doesn't exist.", iso_path.display());
            } else if output_path.exists() {
                eprintln!("Error: {} already exists.", output_path.display());
            } else {
                match Game::open(iso_path) {
                    Ok(ref mut game) => {
                        if let Err(..) = game.extract(output_path) {
                            eprintln!("Failed to write files.");
                        }
                    },
                    Err(..) => eprintln!("Invalid iso: {}.", iso_path.display()),
                }
            }
        },
        ("info", Some(cmd)) => {
            let iso_path = Path::new(cmd.value_of("path_to_iso").unwrap());
            if !iso_path.exists() {
                eprintln!("Error: the iso {} doesn't exist.", iso_path.display());
            }
            match Game::open(iso_path) {
                Ok(ref mut game) => {
                    game.print_info();
                },
                Err(..) => eprintln!("Invalid iso: {}.", iso_path.display()),
            }
        },
        _ => (),
    }
}

