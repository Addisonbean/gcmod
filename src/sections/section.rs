use crate::NumberStyle;
use std::cmp::Ordering::*;
use std::cmp::Ordering;

pub trait Section {
    fn print_info(&self, style: NumberStyle);

    fn start(&self) -> u64;

    fn size(&self) -> usize;

    fn end(&self) -> u64 {
        self.start() + self.size() as u64 - 1
    }

    fn compare_offset(&self, offset: u64) -> Ordering {
        if self.end() < offset {
            Less
        } else if self.start() > offset {
            Greater
        } else {
            Equal
        }
    }
}
