use std::io::{self, Write};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::cmp::Ordering::*;
use std::borrow::Cow;
use std::fmt;

use ::{Extract, ReadSeek};

#[derive(Debug)]
pub struct LayoutSection<'a, 'b> {
    pub name: Cow<'a, str>,
    pub section_type: &'static str,
    pub start: u64,
    pub end: u64,
    section: &'b Extract,
}

impl<'a, 'b> LayoutSection<'a, 'b> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        section_type: &'static str,
        start: u64,
        len: usize,
        section: &'b Extract,
    ) -> LayoutSection<'a, 'b> {
        let end = start + len as u64 - if len == 0 { 0 } else { 1 };
        LayoutSection {
            name: name.into(),
            section_type,
            start,
            end,
            section,
        }
    }

    pub fn len(&self) -> usize {
        (self.end - self.start) as usize + 1
    }

    pub fn compare_offset(&self, offset: u64) -> Ordering {
        if offset < self.start {
            Less
        } else if offset > self.end {
            Greater
        } else {
            Equal
        }
    }

    pub fn contains_offset(&self, offset: u64) -> bool {
        self.compare_offset(offset) == Equal
    }
}

impl<'a, 'b> Extract for LayoutSection<'a, 'b> {
    fn extract(&self, iso: &mut ReadSeek, output: &mut Write) -> io::Result<()> {
        self.section.extract(iso, output)
    }
}

impl<'a, 'b> PartialEq for LayoutSection<'a, 'b> {
    fn eq(&self, other: &LayoutSection) -> bool {
        self.start == other.start
    }
}

impl<'a, 'b> Eq for LayoutSection<'a, 'b> {}

impl<'a, 'b> PartialOrd for LayoutSection<'a, 'b> {
    fn partial_cmp(&self, other: &LayoutSection) -> Option<Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl<'a, 'b> Ord for LayoutSection<'a, 'b> {
    fn cmp(&self, other: &LayoutSection) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl<'a, 'b> fmt::Display for LayoutSection<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "Type: {}", self.section_type)?;
        writeln!(f, "Start: {}", self.start)?;
        writeln!(f, "End: {}", self.end)?;
        write!(f, "Size: {} bytes", self.len())
    }
}

