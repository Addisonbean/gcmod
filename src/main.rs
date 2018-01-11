extern crate gamecube_iso_assistant;
extern crate rand;

use std::env;
use std::process::exit;

use gamecube_iso_assistant::Game;

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

    //*
    let mut tmp_dir_name = env::temp_dir();
    tmp_dir_name.push(&format!("gamecube_dol-{}", rand::random::<u32>()));
    game.write_dol(&tmp_dir_name).unwrap();
    // */


}

