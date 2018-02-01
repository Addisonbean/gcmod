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
            extract_iso(cmd.value_of("path_to_iso").unwrap(),
                        cmd.value_of("output_dir").unwrap());
        },
        ("info", Some(cmd)) => {
            print_iso_info(cmd.value_of("path_to_iso").unwrap());
        },
        _ => (),
    }
}

fn extract_iso<P>(input: P, output: P)
    where P: AsRef<Path>
{
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

fn print_iso_info<P: AsRef<Path>>(input: P)
    where P: AsRef<Path>
{
    try_to_open_game(input).as_ref().map(Game::print_info);
}

fn try_to_open_game<P>(path: P) -> Option<Game>
    where P: AsRef<Path>
{
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

