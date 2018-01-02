use std::fs::File;
use std::io::{BufReader, BufRead, Read, Seek, SeekFrom};

pub enum FSTEntry {
    File { filename_offset: usize, file_offset: usize, length: usize },

    /*
     * `next_index` is the index of the next entry that's not in the directory.
     * For the root, this happens to be the amount of entries in the FST
     */
    Directory { filename_offset: usize, parent_index: usize, next_index: usize },
}

pub struct Game {
    pub game_id: String,
    pub title: String,
}

impl Game {
    pub fn open(filename: &str) -> Option<Game> {
        let f = match File::open(&filename) {
            Ok(f) => f,
            Err(_) => return None,
        };
        let mut the_reads = BufReader::new(f);
        let mut game_id = String::with_capacity(6);
        let mut title = String::with_capacity(0x60);

        // the_reads.take(6).read_to_string(&mut game_id);
        // TODO: find out why the line below works (cause the `&mut`)
        (&mut the_reads).take(6).read_to_string(&mut game_id).unwrap();

        the_reads.seek(SeekFrom::Start(0x20));
        (&mut the_reads).take(0x60).read_to_string(&mut title).unwrap();

        Some(Game {
            game_id,
            title,
        })
    }

}

