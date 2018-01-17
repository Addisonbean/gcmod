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
                    Some(ref mut game) => {
                        if let Err(..) = game.write_files(output_path) {
                            eprintln!("Failed to write files.");
                        }
                    },
                    None => eprintln!("Invalid iso: {}.", iso_path.display()),
                }
            }
            // println!("do that extracting..., {:?}", cmd.value_of("filename"));
        },
        _ => (),
    }
}

/*
fn main() {
    let filename = match env::args().nth(1) {
        Some(s) => s,
        None => {
            eprintln!("Please provide a file to open.");
            exit(1);
        },
    };

    let mut game = match Game::open(&filename) {
        Some(g) => g,
        None => {
            eprintln!("Could not open '{}'", &filename);
            exit(1);
        },
    };

    println!("{}", &game.title);
    println!("{}", &game.game_id);
    println!("{}", game.fst.len());

    for e in &game.fst[0..3] {
        println!("{}", e.info().name);
    }

    println!("{}", &game.fst.last().unwrap().info().name);

    /*
    let mut tmp_dir_name = env::temp_dir();
    tmp_dir_name.push(&format!("gamecube_rom-{}", rand::random::<u32>()));
    game.write_files(tmp_dir_name).unwrap();
    // */

    /*
    let mut tmp_dir_name = env::temp_dir();
    tmp_dir_name.push(&format!("gamecube_dol-{}", rand::random::<u32>()));
    game.write_dol(&tmp_dir_name).unwrap();
    // */

    //*
    let mut tmp_name = env::temp_dir();
    tmp_name.push(&format!("gamecube_app_loader-{}", rand::random::<u32>()));
    game.write_app_loader(&tmp_name).unwrap();
    // */
    
    println!("{:?}", AppLoader::new(&mut File::open(&tmp_name).unwrap()));

}
*/

