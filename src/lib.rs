extern crate byteorder;

mod game;
pub use game::Game;

pub mod fst;
pub mod dol;
pub mod app_loader;

pub const WRITE_CHUNK_SIZE: usize = 1048576; // 1048576 = 2^20 = 1MiB

