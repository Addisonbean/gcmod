pub mod segment;

use std::borrow::Cow;
use std::cmp::max;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::iter::Iterator;

use byteorder::{BigEndian, ReadBytesExt};

use layout_section::{
    LayoutSection,
    SectionType,
    UniqueLayoutSection,
    UniqueSectionType,
};
use ::extract_section;

use self::segment::{Segment, SegmentType};

const TEXT_SEG_COUNT: usize = 7;
const DATA_SEG_COUNT: usize = 11;

pub const DOL_OFFSET_OFFSET: u64 = 0x0420;
pub const DOL_HEADER_LEN: usize = 0x100;

#[derive(Debug)]
pub struct DOLHeader {
    pub offset: u64,
    pub text_segments: [Segment; TEXT_SEG_COUNT],
    pub data_segments: [Segment; DATA_SEG_COUNT],
    pub dol_size: usize,
    pub entry_point: u64,
}

impl DOLHeader {
    pub fn new<R>(mut file: R, offset: u64) -> io::Result<DOLHeader>
    where
        R: Read + Seek,
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
                    seg_type[i].offset = 
                        offset + file.read_u32::<BigEndian>()? as u64;
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

    pub fn find_segment(
        &self,
        seg_type: SegmentType,
        number: u64,
    ) -> Option<&Segment> {
        use self::segment::SegmentType::*;
        match seg_type {
            Text => &self.text_segments[..],
            Data => &self.data_segments[..],
        }.get(number as usize)
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = &Segment> {
        self.text_segments.iter().chain(self.data_segments.iter())
    }

    pub fn extract<R, W>(mut iso: R, dol_addr: u64, file: W) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
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

impl<'a> LayoutSection<'a> for DOLHeader {
    fn name(&self) -> Cow<'static, str> {
        "&&systemdata/Start.dol".into()
    }

    fn section_type(&self) -> SectionType {
        SectionType::DOLHeader
    }

    fn len(&self) -> usize {
        DOL_HEADER_LEN
    }

    fn start(&self) -> u64 {
        self.offset
    }

    fn print_info(&self) {
        println!("Offset: {}", self.offset);
        println!("Size: {} bytes", self.dol_size);
        println!("Header Size: {} bytes", DOL_HEADER_LEN);
        println!("Entry point: {}", self.entry_point);
    }
}

impl<'a> UniqueLayoutSection<'a> for DOLHeader {
    fn section_type(&self) -> UniqueSectionType {
        UniqueSectionType::DOLHeader
    }

    fn with_offset<R>(file: R, offset: u64) -> io::Result<DOLHeader>
    where
        Self: Sized,
        R: BufRead + Seek,
    {
        DOLHeader::new(file, offset)
    }
}

