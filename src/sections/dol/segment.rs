use std::borrow::Cow;

use regex::Regex;

use sections::layout_section::{LayoutSection, SectionType};
use ::{format_u64, format_usize, NumberStyle, parse_as_u64};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum SegmentType {
    Text, Data
}

impl SegmentType {
    pub fn to_string(self, seg_num: u64) -> String {
        use self::SegmentType::*;
        match self {
            Text => format!(".text{}", seg_num),
            Data => format!(".data{}", seg_num),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Segment {
    // NOTE: `offset` is not the offset stored on the ROM.
    // The ROM provides the offset relative to the start of the DOL header,
    // whereas this is relative to the beginning of the ROM. This offset
    // is essentially the offset relative to the DOL (which is the value
    // given in the ROM), plus the offset of the DOL itself.
    pub offset: u64,
    pub size: usize,
    pub loading_address: u64,
    pub seg_type: SegmentType,
    pub seg_num: u64,
}

impl Segment {
    pub fn text() -> Segment {
        Segment {
            offset: 0,
            size: 0,
            loading_address: 0,
            seg_type: SegmentType::Text,
            seg_num: 0,
        }
    }

    pub fn data() -> Segment {
        Segment {
            offset: 0,
            size: 0,
            loading_address: 0,
            seg_type: SegmentType::Data,
            seg_num: 0,
        }
    }

    pub fn to_string(self) -> String {
        self.seg_type.to_string(self.seg_num)
    }

    pub fn parse_segment_name(name: &str) -> Option<(SegmentType, u64)> {
        use self::SegmentType::*;
        lazy_static! {
            static ref SEG_NAME_REGEX: Regex =
                Regex::new(r"^\.?(text|data)(\d+)$").unwrap();
        }
        SEG_NAME_REGEX.captures(name).and_then(|c| {
            parse_as_u64(c.get(2).unwrap().as_str()).map(|n| {
                let t = match c.get(1).unwrap().as_str() {
                    "text" => Text,
                    "data" => Data,
                    _ => unreachable!(),
                };
                (t, n)
            }).ok()
        })
    }
}

impl<'a> LayoutSection<'a> for Segment {
    fn name(&self) -> Cow<'static, str> {
        self.to_string().into()
    }

    fn section_type(&self) -> SectionType {
        SectionType::DOLSegment
    }

    fn len(&self) -> usize {
        self.size
    }

    fn start(&self) -> u64 {
        self.offset
    }

    fn print_info(&self, style: NumberStyle) {
        println!("Segment name: {}", self.seg_type.to_string(self.seg_num));
        println!("Offset: {}", format_u64(self.offset, style));
        println!("Size: {}", format_usize(self.size, style));
        println!("Loading address: {}", format_u64(self.loading_address, style));
    }
}
