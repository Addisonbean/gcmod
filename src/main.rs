extern crate gamecube_iso_assistant;
extern crate rand;

use std::env;
use std::process::exit;
use std::fs::File;

use gamecube_iso_assistant::Game;
use gamecube_iso_assistant::app_loader::AppLoader;

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

