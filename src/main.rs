use std::env;
use std::process::exit;

mod game;
use game::Game;

fn main() {
    let filename = match env::args().nth(1) {
        Some(s) => s,
        None => {
            eprintln!("Please provide a file to open.");
            exit(1);
        },
    };

    let game = match Game::open(&filename) {
        Some(g) => g,
        None => {
            eprintln!("Could not open '{}'", &filename);
            exit(1);
        },
    };

    println!("{}", &game.title);
    // println!("{}", &game.title.capacity());
    println!("{}", &game.game_id);
    // println!("{}", &game.game_id.capacity());
}
