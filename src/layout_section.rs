use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::cmp::Ordering::*;
use std::marker::PhantomData;
use std::borrow::Cow;
use std::fmt;

#[derive(Debug)]
pub struct LayoutSection<'a> {
    pub name: Cow<'a, str>,
    pub section_type: &'static str,
    pub start: u64,
    pub end: u64,
    _private_field: PhantomData<()>,
}

impl<'a> LayoutSection<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        section_type: &'static str,
        start: u64,
        len: usize
    ) -> LayoutSection<'a> {
        let end = start + len as u64 - if len == 0 { 0 } else { 1 };
        LayoutSection {
            name: name.into(),
            section_type,
            start,
            end,
            _private_field: PhantomData,
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

impl<'a> PartialEq for LayoutSection<'a> {
    fn eq(&self, other: &LayoutSection) -> bool {
        self.start == other.start
    }
}

impl<'a> Eq for LayoutSection<'a> {}

impl<'a> PartialOrd for LayoutSection<'a> {
    fn partial_cmp(&self, other: &LayoutSection) -> Option<Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl<'a> Ord for LayoutSection<'a> {
    fn cmp(&self, other: &LayoutSection) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl<'a> fmt::Display for LayoutSection<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}\nType: {}\nStart: {}\nEnd: {}\nSize: {} bytes",
               self.name, self.section_type, self.start, self.end, self.len())
    }
}
