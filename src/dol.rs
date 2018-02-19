use std::io::{self, Read, Seek, SeekFrom, Write};
use std::cmp::max;

use byteorder::{ReadBytesExt, BigEndian};

use ::write_section;

const TEXT_SEG_COUNT: usize = 7;
const DATA_SEG_COUNT: usize = 11;

pub const DOL_OFFSET_OFFSET: u64 = 0x0420; 

#[derive(Copy, Clone, Debug)]
pub enum SegmentType {
    Text, Data
}

impl SegmentType {
    pub fn to_string(self, seg_num: usize) -> String {
        use self::SegmentType::*;
        match self {
            Text => format!(".text{}", seg_num),
            Data => format!(".data{}", seg_num),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Segment {
    // the start of the segment is relative to the beginning of the DOL section
    pub start: u64,
    pub size: u64,
    pub loading_address: u64,
    pub seg_type: SegmentType,
}

impl Segment {
    pub fn text() -> Segment {
        Segment {
            start: 0,
            size: 0,
            loading_address: 0,
            seg_type: SegmentType::Text,
        }
    }

    pub fn data() -> Segment {
        Segment {
            start: 0,
            size: 0,
            loading_address: 0,
            seg_type: SegmentType::Data,
        }
    }
}

#[derive(Debug)]
pub struct DOLHeader {
    pub text_segments: [Segment; TEXT_SEG_COUNT],
    pub data_segments: [Segment; DATA_SEG_COUNT],
    pub dol_size: usize,
    pub entry_point: u64,
}

impl DOLHeader {
    pub fn new<F: Read + Seek>(file: &mut F) -> io::Result<DOLHeader> {
        let mut text_segments = [Segment::text(); TEXT_SEG_COUNT];
        let mut data_segments = [Segment::data(); DATA_SEG_COUNT];
        {
            let mut segs = [
                &mut text_segments[..],
                &mut data_segments[..],
            ];

            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].start = file.read_u32::<BigEndian>()? as u64;
                }
            }

            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].loading_address =
                        file.read_u32::<BigEndian>()? as u64;
                }
            }

            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].size = file.read_u32::<BigEndian>()? as u64;
                }
            }
        }

        file.seek(SeekFrom::Current(8))?;
        let entry_point = file.read_u32::<BigEndian>()? as u64;

        let dol_size = text_segments.iter().chain(data_segments.iter())
            .map(|s| s.start + s.size).max().unwrap() as usize;

        Ok(DOLHeader {
            text_segments,
            data_segments,
            dol_size,
            entry_point,
        })
    }

    pub fn write_to_disk<R: Read + Seek, W: Write>(
        iso: &mut R,
        dol_addr: u64,
        file: &mut W
    ) -> io::Result<()> {
        iso.seek(SeekFrom::Start(dol_addr))?;
        let mut dol_size = 0;

        // 7 code segments
        for i in 0..7 {
            iso.seek(SeekFrom::Start(dol_addr + 0x00 + i * 4))?;
            let seg_offset = iso.read_u32::<BigEndian>()?;

            iso.seek(SeekFrom::Start(dol_addr + 0x90 + i * 4))?;
            let seg_size = iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        // 11 data segments
        for i in 0..11 {
            iso.seek(SeekFrom::Start(dol_addr + 0x1c + i * 4))?;
            let seg_offset = iso.read_u32::<BigEndian>()?;

            iso.seek(SeekFrom::Start(dol_addr + 0xac + i * 4))?;
            let seg_size = iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        iso.seek(SeekFrom::Start(dol_addr))?;

        write_section(iso, dol_size as usize, file)
    }
}

