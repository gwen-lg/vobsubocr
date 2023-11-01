use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Debug},
    fs::File,
    io::{BufReader, Read, Seek},
};

use super::{CompositionState, Error, WindowInformationObject};
use crate::pgs::{read_window_info, u24::u24};

const MAGIC_NUMBER: [u8; 2] = [0x50, 0x47];

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SegmentTypeCode(u8);
impl From<u8> for SegmentTypeCode {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
impl fmt::Debug for SegmentTypeCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let friendly = match *self {
            Self::PDS => "PDS",
            Self::ODS => "ODS",
            Self::PCS => "PCS",
            Self::WDS => "WDS",
            Self::END => "END",
            _ => "<Invalid>",
        };
        write!(f, "{}-{}", self.0, friendly)
    }
}
impl fmt::Display for SegmentTypeCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let friendly = match *self {
            Self::PDS => "PDS",
            Self::ODS => "ODS",
            Self::PCS => "PCS",
            Self::WDS => "WDS",
            Self::END => "END",
            _ => "<Invalid>",
        };
        write!(f, "{}", friendly)
    }
}

impl SegmentTypeCode {
    pub(crate) const PDS: SegmentTypeCode = SegmentTypeCode(0x14);
    pub(crate) const ODS: SegmentTypeCode = SegmentTypeCode(0x15);
    pub(crate) const PCS: SegmentTypeCode = SegmentTypeCode(0x16);
    pub(crate) const WDS: SegmentTypeCode = SegmentTypeCode(0x17);
    pub(crate) const END: SegmentTypeCode = SegmentTypeCode(0x80);
}

#[derive(Debug)]
pub enum SegmentType {
    Pds(PaletteDefinitionSegment),
    Ods(ObjectDefinitionSegment),
    Pcs(PresentationCompositionSegment),
    Wds(WindowDefinitionSegment),
    End,
}
impl SegmentType {
    fn _value(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }
}

impl fmt::Display for SegmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = match self {
            SegmentType::Pds(_) => "Pds",
            SegmentType::Ods(_) => "Ods",
            SegmentType::Pcs(_) => "Pcs",
            SegmentType::Wds(_) => "Wds",
            SegmentType::End => "End",
        };
        write!(f, "{info}")
    }
}

#[derive(Debug)]
pub struct SegmentHeader {
    pts: u32,
    dts: u32,
    type_code: SegmentTypeCode,
    size: u16,
}
impl SegmentHeader {
    pub fn presentation_time(&self) -> u32 {
        let time_ms = self.pts / 90;
        time_ms
    }
    pub fn type_code(&self) -> SegmentTypeCode {
        self.type_code
    }
    pub fn size(&self) -> u16 {
        self.size
    }
}

impl fmt::Display for SegmentHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let presentation_time = self.presentation_time();
        let seg_type = self.type_code;
        let size = self.size;
        // dts is ignored as always 0 ?????
        write!(
            f,
            "{{ Presentation: {presentation_time}, seg_type: {seg_type}, size: {size} }}"
        )
    }
}
pub fn read_header(reader: &mut BufReader<File>) -> Result<SegmentHeader, Error> {
    const HEADER_LEN: usize = 2 + 4 + 4 + 1 + 2;
    let mut header_buf = [0; HEADER_LEN];
    reader.read_exact(&mut header_buf)?;

    //buffer = buf_next;
    if header_buf[0..2] != MAGIC_NUMBER {
        let file_idx = reader.stream_position().unwrap();
        let msg = format!(
            "Unable to read segment header - MAGIC_NUMBER missing! Stream pos : {file_idx}"
        );
        return Err(msg.into());
    }
    let pts = u32::from_be_bytes(header_buf[2..6].try_into().unwrap());
    let dts = u32::from_be_bytes(header_buf[6..10].try_into().unwrap());
    let type_code = SegmentTypeCode::from(header_buf[10]);
    let size = u16::from_be_bytes(header_buf[11..13].try_into().unwrap());

    Ok(SegmentHeader {
        pts,
        dts,
        type_code,
        size,
    })
}

#[derive(Debug)]
pub struct PresentationCompositionSegment {
    width: u16,                          // Video width in pixels (ex. 0x780 = 1920)
    height: u16,                         // Video height in pixels (ex. 0x438 = 1080)
    frame_rate: u8,                      // Always 0x10. Can be ignored.
    composition_number: u16, // Number of this specific composition. It is incremented by one every time a graphics update occurs.
    composition_state: CompositionState, // Type of this composition. Allowed values are:
    // 0x00: Normal | 0x40: Acquisition Point | 0x80: Epoch Start
    palette_update_flag: u8, //	Indicates if this PCS describes a Palette only Display Update. Allowed values are: 0x00: False | 0x80: True
    palette_id: u8,          // ID of the palette to be used in the Palette only Display Update
    composition_objects: Vec<WindowInformationObject>, // Number of composition objects defined in this segment
}
pub fn read_pcs(reader: &mut BufReader<File>) -> Result<PresentationCompositionSegment, Error> {
    const PCS_LEN: usize = 2 + 2 + 1 + 2 + 1 + 1 + 1 + 1; //size_of::<Pcs>();
    let mut pcs_buf = [0; PCS_LEN];
    reader.read_exact(&mut pcs_buf)?;

    let width = u16::from_be_bytes(pcs_buf[0..2].try_into().unwrap());
    let height = u16::from_be_bytes(pcs_buf[2..4].try_into().unwrap());
    let frame_rate = pcs_buf[4];
    assert!(frame_rate == 0x10);
    let composition_number = u16::from_be_bytes(pcs_buf[5..7].try_into().unwrap());
    let composition_state = pcs_buf[7].try_into()?;
    // if composition_state != 0x00 && composition_state != 0x40 && composition_state != 0x80 {
    //     // 0x00: Normal | 0x40: Acquisition Point | 0x80: Epoch Start
    //     return Err(String::from("TODO composition_state").into());
    // }
    let palette_update_flag = pcs_buf[8];
    if palette_update_flag != 0x00 && palette_update_flag != 0x80 {
        //	Indicates if this PCS describes a Palette only Display Update. Allowed values are: 0x00: False | 0x80: True
        return Err(String::from("TODO palette_update_flag").into());
    }
    let palette_id = pcs_buf[9];
    let number_of_composition_objects = pcs_buf[10];
    let range = 0..number_of_composition_objects;
    let composition_objects: Result<Vec<_>, _> = range.map(|_| read_window_info(reader)).collect();
    let composition_objects = composition_objects?;

    Ok(PresentationCompositionSegment {
        width,
        height,
        frame_rate,
        composition_number,
        composition_state,
        palette_update_flag,
        palette_id,
        composition_objects,
    })
}

#[derive(Debug)]
pub struct WindowDefinitionSegment {
    number_of_windows: u8,
    window_id: u8,
    window_horizontal_position: u16,
    window_vertical_position: u16,
    window_width: u16,
    window_height: u16,
}

pub fn read_wds(reader: &mut BufReader<File>) -> Result<WindowDefinitionSegment, Error> {
    const WDS_LEN: usize = 1 + 1 + 2 + 2 + 2 + 2; //size_of::<WindowDefinitionSegment>();
    let mut wds_buf = [0; WDS_LEN];
    reader.read_exact(&mut wds_buf)?;

    let number_of_windows = wds_buf[0];
    let window_id = wds_buf[1];
    let window_horizontal_position = u16::from_be_bytes(wds_buf[2..4].try_into().unwrap());
    let window_vertical_position = u16::from_be_bytes(wds_buf[4..6].try_into().unwrap());
    let window_width = u16::from_be_bytes(wds_buf[6..8].try_into().unwrap());
    let window_height = u16::from_be_bytes(wds_buf[8..10].try_into().unwrap());
    Ok(WindowDefinitionSegment {
        number_of_windows,
        window_id,
        window_horizontal_position,
        window_vertical_position,
        window_width,
        window_height,
    })
}

#[derive(Debug)]
pub struct PaletteEntry {
    palette_entry_id: u8,      // Entry number of the palette
    luminance: u8,             // Luminance (Y value)
    color_difference_red: u8,  // Color Difference Red (Cr value)
    color_difference_blue: u8, // Color Difference Blue (Cb value)
    transparency: u8,          // Transparency (Alpha value)
}
#[derive(Debug)]
pub struct PaletteDefinitionSegment {
    palette_id: u8,             // ID of the palette
    palette_version_number: u8, //	Version of this palette within the Epoch
    palette_entries: Vec<PaletteEntry>,
}

pub fn read_pds(
    reader: &mut BufReader<File>,
    segments_size: usize,
) -> Result<PaletteDefinitionSegment, Error> {
    //const PDS_LEN: usize = 7; //size_of::<PaletteDefinitionSegment>();
    let mut pds_buf = vec![0; segments_size.into()];
    reader.read_exact(&mut pds_buf)?;

    let palette_id = pds_buf[0];
    let palette_version_number = pds_buf[1];

    let nb_palette_entry: usize = (segments_size - 2) / 5;
    assert_eq!((nb_palette_entry * 5) + 2, segments_size);
    let range = 0..nb_palette_entry;
    let palette_entries = range
        .map(|idx| {
            let offset = 2 + (idx * 5);
            PaletteEntry {
                palette_entry_id: pds_buf[offset],
                luminance: pds_buf[offset + 1],
                color_difference_red: pds_buf[offset + 2],
                color_difference_blue: pds_buf[offset + 3],
                transparency: pds_buf[offset + 4],
            }
        })
        .collect();
    Ok(PaletteDefinitionSegment {
        palette_id,
        palette_version_number,
        palette_entries,
    })
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum LastInSequenceFlag {
    LastInSequence = 0x40,
    FirstInSequence = 0x80,
    FirstAndLastInSequence = 0xC0,
}
impl TryFrom<u8> for LastInSequenceFlag {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x40 => Ok(LastInSequenceFlag::LastInSequence),
            0x80 => Ok(LastInSequenceFlag::FirstInSequence),
            0xC0 => Ok(LastInSequenceFlag::FirstAndLastInSequence),
            _ => Err(Error::String {
                value: "LastInSequenceFlag parsing error".into(),
            }),
        }
    }
}

#[derive(Debug)]
pub struct ObjectDefinitionSegment {
    object_id: u16,
    object_version_number: u8,
    last_in_sequence_flag: LastInSequenceFlag,
    object_data_lenght: u24,
    width: u16,
    height: u16,
    object_data_seek: u64, //Vec<u8>, // ????
    object_data_len: usize,
}
// impl Debug for ObjectDefinitionSegment {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let object_id = self.object_id;
//         let object_version_number = self.object_version_number;
//         let last_in_sequence_flag = self.last_in_sequence_flag;
//         let object_data_lenght = self.object_data_lenght;
//         let width = self.width;
//         let height = self.height;
//         let object_data_seek = self.object_data_seek;
//         let object_data_len = self.object_data_len;
//         write!(
//             f,
//             "ObjectDefinitionSegment {{ object_id: {object_id}, \
//         object_version_number: {object_version_number}, \
//         last_in_sequence_flag: {last_in_sequence_flag:?}, \
//         object_data_lenght: {object_data_lenght:?}, \
//         width: {width}, \
//         height: {height}, \
//         object_data: [_;{object_data_len}] }}"
//         )
//     }
// }

pub fn read_ods(
    reader: &mut BufReader<File>,
    segments_size: usize,
) -> Result<ObjectDefinitionSegment, Error> {
    const ODS_HEADER: usize = 2 + 1 + 1 + 3 + 2 + 2; //size_of::<PaletteDefinitionSegment>();
    let mut ods_buf = [0; ODS_HEADER];
    reader.read_exact(&mut ods_buf)?;

    let object_id = u16::from_be_bytes(ods_buf[0..2].try_into().unwrap());
    let object_version_number = ods_buf[2];
    let last_in_sequence_flag = ods_buf[3].try_into()?;

    let object_data_lenght =
        u24::from(<&[u8] as TryInto<[u8; 3]>>::try_into(&ods_buf[4..7]).unwrap());
    let width = u16::from_be_bytes(ods_buf[7..9].try_into().unwrap());
    let height = u16::from_be_bytes(ods_buf[9..11].try_into().unwrap());
    let data_size: usize = object_data_lenght.to_u32().try_into().unwrap();
    let data_size = data_size - 4; // don't know why for now !!!

    //object_data.resize(data_size, 0);
    assert!(ODS_HEADER + data_size == segments_size);
    let data_cursor = reader.stream_position()?;
    let mut buff = vec![0; data_size];
    reader.read_exact(&mut buff)?;
    //reader.consume(data_size);

    Ok(ObjectDefinitionSegment {
        object_id,
        object_version_number,
        last_in_sequence_flag,
        object_data_lenght,
        width,
        height,
        object_data_seek: data_cursor,
        object_data_len: data_size,
    })
}
