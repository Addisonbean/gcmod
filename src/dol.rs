use std::io::{self, Read};

use byteorder::{ReadBytesExt, BigEndian};

const TEXT_SEG_COUNT: usize = 7;
const DATA_SEG_COUNT: usize = 11;

pub const DOL_OFFSET_OFFSET: u64 = 0x0420; 

#[derive(Copy, Clone, Debug, Default)]
pub struct Segment {
    // the start of the segment is relative to the beginning of the DOL section
    pub start: u64,
    pub size: u64,
}

#[derive(Debug)]
pub struct Header {
    pub text_segments: [Segment; TEXT_SEG_COUNT],
    pub data_segments: [Segment; DATA_SEG_COUNT],
}

impl Header {
    pub fn new<F: Read>(file: &mut F) -> io::Result<Header> {
        let mut text_segments = [Segment::default(); TEXT_SEG_COUNT];
        let mut data_segments = [Segment::default(); DATA_SEG_COUNT];
        {
            let mut segs = [
                &mut text_segments[..],
                &mut data_segments[..],
            ];

            // for &mut seg_type in segs {
            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].start = file.read_u32::<BigEndian>()? as u64;
                }
            }

            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].size = file.read_u32::<BigEndian>()? as u64;
                }
            }
        }

        Ok(Header {
            text_segments,
            data_segments,
        })
    }
}

