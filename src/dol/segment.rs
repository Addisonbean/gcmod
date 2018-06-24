use std::borrow::Cow;

use layout_section::{LayoutSection, SectionType};

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
    // whereas this is relative to the beginning of the ROM. It is essentially
    // the offset relative to the DOL which is given in the ROM,
    // plus the offset of the DOL itself.
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

    fn print_info(&self) {
        println!("Segment id: {}", self.seg_num);
        println!("Segment type: {}", self.seg_type.to_string(self.seg_num));
        println!("Offset: {}", self.offset);
        println!("Size: {}", self.size);
    }
}
