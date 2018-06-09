use std::io::{self, Read, Seek, SeekFrom, Write};
use std::cmp::max;
use std::iter::Iterator;

use byteorder::{ReadBytesExt, BigEndian};

use layout_section::LayoutSection;
use ::{Extract, extract_section, ReadSeek};

const TEXT_SEG_COUNT: usize = 7;
const DATA_SEG_COUNT: usize = 11;

pub const DOL_OFFSET_OFFSET: u64 = 0x0420;
pub const DOL_HEADER_LEN: usize = 0x100;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
        self.seg_type.to_string(self.seg_num as usize)
    }
}

#[derive(Debug)]
pub struct DOLHeader {
    pub offset: u64,
    pub text_segments: [Segment; TEXT_SEG_COUNT],
    pub data_segments: [Segment; DATA_SEG_COUNT],
    pub dol_size: usize,
    pub entry_point: u64,
}

impl DOLHeader {
    pub fn new<F>(file: &mut F, offset: u64) -> io::Result<DOLHeader> 
        where F: Read + Seek
    {
        file.seek(SeekFrom::Start(offset))?;
        let mut text_segments = [Segment::text(); TEXT_SEG_COUNT];
        let mut data_segments = [Segment::data(); DATA_SEG_COUNT];
        {
            let mut segs = [
                &mut text_segments[..],
                &mut data_segments[..],
            ];

            for ref mut seg_type in segs.iter_mut() {
                for i in 0..seg_type.len() {
                    seg_type[i].offset = offset + file.read_u32::<BigEndian>()? as u64;
                    seg_type[i].seg_num = i as u64;
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
                    seg_type[i].size = file.read_u32::<BigEndian>()? as usize;
                }
            }
        }

        file.seek(SeekFrom::Current(8))?;
        let entry_point = file.read_u32::<BigEndian>()? as u64;

        let dol_size = text_segments.iter().chain(data_segments.iter())
            .map(|s| (s.offset - offset) as usize + s.size).max().unwrap();

        Ok(DOLHeader {
            offset,
            text_segments,
            data_segments,
            dol_size,
            entry_point,
        })
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = &Segment> {
        self.text_segments.iter().chain(self.data_segments.iter())
    }

    pub fn extract<R: Read + Seek, W: Write>(
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

        extract_section(iso, dol_size as usize, file)
    }
}

impl<'a, 'b> From<&'b DOLHeader> for LayoutSection<'a, 'b> {
    fn from(d: &'b DOLHeader) -> LayoutSection<'a, 'b> {
        LayoutSection::new(
            "&&systemdata/Start.dol",
            "DOL Header",
            d.offset,
            DOL_HEADER_LEN,
            d,
        )
    }
}

impl<'a, 'b> From<&'b Segment> for LayoutSection<'a, 'b> {
    fn from(s: &'b Segment) -> LayoutSection<'a, 'b> {
        LayoutSection::new(
            s.to_string(),
            "DOL Segment",
            s.offset,
            s.size,
            s,
        )
    }
}

impl Extract for DOLHeader {
    fn extract(&self, iso: &mut ReadSeek, output: &mut Write) -> io::Result<()> {
        iso.seek(SeekFrom::Start(self.offset))?;
        extract_section(iso, DOL_HEADER_LEN, output)
    }
}

impl Extract for Segment {
    fn extract(
        &self,
        iso: &mut ReadSeek,
        output: &mut Write,
    ) -> io::Result<()> {
        iso.seek(SeekFrom::Start(self.offset))?;
        extract_section(iso, self.size, output)
    }
}

