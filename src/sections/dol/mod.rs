pub mod segment;

use std::borrow::Cow;
use std::cmp::max;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::iter::Iterator;

use byteorder::{BigEndian, ReadBytesExt};

use sections::layout_section::{
    LayoutSection,
    SectionType,
    UniqueLayoutSection,
    UniqueSectionType,
};
use ::{extract_section, format_u64, format_usize, NumberStyle};

use self::segment::{Segment, SegmentType};

const TEXT_SEG_COUNT: usize = 7;
const DATA_SEG_COUNT: usize = 11;
const TOTAL_SEG_COUNT: usize = TEXT_SEG_COUNT + DATA_SEG_COUNT;

pub const DOL_OFFSET_OFFSET: u64 = 0x0420;
pub const DOL_HEADER_LEN: usize = 0x100;

#[derive(Debug)]
pub struct DOLHeader {
    pub offset: u64,
    pub dol_size: usize,
    pub entry_point: u64,
    segments: Vec<Segment>,
    // This is the index in `segments` where the data segments are. The segments
    // before this index are all text segments.
    data_segments_index: usize,
}

impl DOLHeader {
    pub fn new<R>(mut file: R, offset: u64) -> io::Result<DOLHeader>
    where
        R: Read + Seek,
    {
        file.seek(SeekFrom::Start(offset + 0x90))?;
        let mut segments = Vec::new();

        let mut data_segments_index = 0;
        let mut is_text = true;
        for i in 0..TOTAL_SEG_COUNT {
            let mut num = i as u64;
            if i >= TEXT_SEG_COUNT {
                is_text = false;
                data_segments_index = segments.len();
                num -= TEXT_SEG_COUNT as u64;
            }
            let size = file.read_u32::<BigEndian>()? as usize;
            if size != 0 {
                let mut s = if is_text {
                    Segment::text()
                } else {
                    Segment::data()
                };
                s.size = size;
                s.seg_num = num;
                segments.push(s);
            }
        }

        file.seek(SeekFrom::Start(offset))?;
        for s in &mut segments[..] {
            let previous = if s.seg_type == SegmentType::Data {
                TEXT_SEG_COUNT as u64
            } else {
                0
            };
            file.seek(SeekFrom::Start(offset + (previous + s.seg_num) * 4))?;
            s.offset = offset + file.read_u32::<BigEndian>()? as u64;
        }

        for s in &mut segments[..] {
            let previous = if s.seg_type == SegmentType::Data {
                TEXT_SEG_COUNT as u64
            } else {
                0
            };
            file.seek(SeekFrom::Start(
                offset + 0x48 + (previous + s.seg_num) * 4
            ))?;
            s.loading_address = file.read_u32::<BigEndian>()? as u64;
        }

        file.seek(SeekFrom::Start(offset + 0xE0))?;
        let entry_point = file.read_u32::<BigEndian>()? as u64;

        let dol_size = segments.iter()
            .map(|s| (s.offset - offset) as usize + s.size).max().unwrap();

        Ok(DOLHeader {
            offset,
            dol_size,
            entry_point,
            segments,
            data_segments_index,
        })
    }

    pub fn find_segment(
        &self,
        seg_type: SegmentType,
        number: u64,
    ) -> Option<&Segment> {
        let start = if seg_type == SegmentType::Data {
            self.data_segments_index
        } else {
            0
        };
        self.segments[start..].iter()
            .find(|s| s.seg_num == number && s.seg_type == seg_type)
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = &Segment> {
        self.segments.iter()
    }

    pub fn extract<R, W>(mut iso: R, file: W, dol_addr: u64) -> io::Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        iso.seek(SeekFrom::Start(dol_addr))?;
        let mut dol_size = 0;

        for i in 0..(TEXT_SEG_COUNT as u64) {
            iso.seek(SeekFrom::Start(dol_addr + 0x00 + i * 4))?;
            let seg_offset = iso.read_u32::<BigEndian>()?;

            iso.seek(SeekFrom::Start(dol_addr + 0x90 + i * 4))?;
            let seg_size = iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        for i in 0..(DATA_SEG_COUNT as u64) {
            iso.seek(SeekFrom::Start(dol_addr + 0x1c + i * 4))?;
            let seg_offset = iso.read_u32::<BigEndian>()?;

            iso.seek(SeekFrom::Start(dol_addr + 0xac + i * 4))?;
            let seg_size = iso.read_u32::<BigEndian>()?;

            dol_size = max(seg_offset + seg_size, dol_size);
        }

        iso.seek(SeekFrom::Start(dol_addr))?;

        extract_section(iso, dol_size as usize, file)
    }

    pub fn segment_at_addr(&self, mem_addr: u64) -> Option<&Segment> {
        self.segments.iter().find(|s|
            s.loading_address <= mem_addr &&
            mem_addr < s.loading_address + s.size as u64
        )
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

    fn print_info(&self, style: NumberStyle) {
        println!("Offset: {}", format_u64(self.offset, style));
        println!("Size: {} bytes", format_usize(self.dol_size, style));
        println!("Header Size: {} bytes", format_usize(DOL_HEADER_LEN, style));
        println!("Entry point: {}", format_u64(self.entry_point, style));
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

