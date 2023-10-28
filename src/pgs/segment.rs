use std::{
    convert::{TryFrom, TryInto},
    fmt,
    fs::File,
    io::Read,
};

use super::{CompositionState, Error};
use crate::pgs::{read_window_info, u24::u24};

const MAGIC_NUMBER: [u8; 2] = [0x50, 0x47];

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum SegmentType {
    Pds = 0x14,
    Ods = 0x15,
    Pcs = 0x16,
    Wds = 0x17,
    End = 0x80,
}
impl SegmentType {
    fn _value(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }
}

//TODO: get a better method ?
impl TryFrom<u8> for SegmentType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x14 => Ok(SegmentType::Pds),
            0x15 => Ok(SegmentType::Ods),
            0x16 => Ok(SegmentType::Pcs),
            0x17 => Ok(SegmentType::Wds),
            0x80 => Ok(SegmentType::End),
            _ => Err("Invalid segment type".into()),
        }
    }
}
impl fmt::Display for SegmentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = match self {
            SegmentType::Pds => "Pds",
            SegmentType::Ods => "Ods",
            SegmentType::Pcs => "Pcs",
            SegmentType::Wds => "Wds",
            SegmentType::End => "End",
        };
        write!(f, "{info}")
    }
}

#[derive(Debug)]
pub struct SegmentHeader {
    pts: u32,
    dts: u32,
    seg_type: SegmentType,
    size: u16,
}
impl SegmentHeader {
    pub fn presentation_time(&self) -> u32 {
        let time_ms = self.pts / 90;
        time_ms
    }
    pub fn sg_type(&self) -> SegmentType {
        self.seg_type
    }
}
impl fmt::Display for SegmentHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let presentation_time = self.presentation_time();
        let seg_type = self.seg_type;
        let size = self.size;
        // dts is ignored as always 0 ?????
        write!(
            f,
            "{{ Presentation: {presentation_time}, seg_type: {seg_type}, size: {size} }}"
        )
    }
}
pub fn read_header(file: &mut File) -> Result<SegmentHeader, Error> {
    const HEADER_LEN: usize = 2 + 4 + 4 + 1 + 2;
    let mut buffer = [0u8; HEADER_LEN];

    let read_count = file.read(&mut buffer)?;
    if read_count < HEADER_LEN {
        return Err(Error::EndOfFile);
    }
    //    let magic_number: [u8; 2]; = [buffer[0..1], ;
    if buffer[0..2] != MAGIC_NUMBER {
        return Err(String::from("Unable to read segment - PG missing!").into());
    }
    let pts = u32::from_be_bytes(buffer[2..6].try_into().unwrap());
    let dts = u32::from_be_bytes(buffer[6..10].try_into().unwrap());
    let seg_type = SegmentType::try_from(buffer[10])?;
    let size = u16::from_be_bytes(buffer[11..13].try_into().unwrap());

    //do_something(&buffer[..read_count]);
    Ok(SegmentHeader {
        pts,
        dts,
        seg_type,
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
    number_of_composition_objects: u8, // Number of composition objects defined in this segment
}
pub fn read_pcs(file: &mut File) -> Result<PresentationCompositionSegment, Error> {
    //println!("Presentation Composition Segment");
    const PCS_LEN: usize = 2 + 2 + 1 + 2 + 1 + 1 + 1 + 1; //size_of::<Pcs>();
    let mut buffer = [0u8; PCS_LEN];
    let read_count = file.read(&mut buffer)?;
    if read_count < PCS_LEN {
        return Err(String::from("Can't read all pc segment").into());
    }

    let width = u16::from_be_bytes(buffer[0..2].try_into().unwrap());
    let height = u16::from_be_bytes(buffer[2..4].try_into().unwrap());
    let frame_rate = buffer[4];
    assert!(frame_rate == 0x10);
    let composition_number = u16::from_be_bytes(buffer[5..7].try_into().unwrap());
    let composition_state = buffer[7].try_into()?;
    // if composition_state != 0x00 && composition_state != 0x40 && composition_state != 0x80 {
    //     // 0x00: Normal | 0x40: Acquisition Point | 0x80: Epoch Start
    //     return Err(String::from("TODO composition_state").into());
    // }
    let palette_update_flag = buffer[8];
    if palette_update_flag != 0x00 && palette_update_flag != 0x80 {
        //	Indicates if this PCS describes a Palette only Display Update. Allowed values are: 0x00: False | 0x80: True
        return Err(String::from("TODO palette_update_flag").into());
    }
    let palette_id = buffer[9];
    let number_of_composition_objects = buffer[10];
    for object_idx in 0..number_of_composition_objects {
        let win_info = read_window_info(file)?;
    }

    Ok(PresentationCompositionSegment {
        width,
        height,
        frame_rate,
        composition_number,
        composition_state,
        palette_update_flag,
        palette_id,
        number_of_composition_objects,
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

pub fn read_wds(file: &mut File) -> Result<WindowDefinitionSegment, Error> {
    const WDS_LEN: usize = 2 + 2 + 1 + 2 + 1 + 1 + 1 + 1; //size_of::<Pcs>();
    let mut buffer = [0u8; WDS_LEN];
    let read_count = file.read(&mut buffer)?;
    if read_count < WDS_LEN {
        return Err(String::from("Can't read all Window Definition Segment").into());
    }

    let number_of_windows = buffer[0];
    let window_id = buffer[1];
    let window_horizontal_position = u16::from_be_bytes(buffer[2..4].try_into().unwrap());
    let window_vertical_position = u16::from_be_bytes(buffer[4..6].try_into().unwrap());
    let window_width = u16::from_be_bytes(buffer[6..8].try_into().unwrap());
    let window_height = u16::from_be_bytes(buffer[8..10].try_into().unwrap());
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
pub struct PaletteDefinitionSegment {
    palette_id: u8,             // ID of the palette
    palette_version_number: u8, //	Version of this palette within the Epoch
    palette_entry_id: u8,       // Entry number of the palette
    luminance: u8,              // Luminance (Y value)
    color_difference_red: u8,   // Color Difference Red (Cr value)
    color_difference_blue: u8,  // Color Difference Blue (Cb value)
    transparency: u8,           // Transparency (Alpha value)
}

pub fn read_pds(file: &mut File) -> Result<PaletteDefinitionSegment, Error> {
    const PDS_LEN: usize = 7; //size_of::<PaletteDefinitionSegment>();
    let mut buffer = [0u8; PDS_LEN];
    let read_count = file.read(&mut buffer)?;
    if read_count < PDS_LEN {
        return Err(String::from("Can't read all Window Definition Segment").into());
    }
    let palette_id = buffer[0];
    let palette_version_number = buffer[1];

    //TODO: can be most than one entry
    let palette_entry_id = buffer[2];
    let luminance = buffer[3];
    let color_difference_red = buffer[4];
    let color_difference_blue = buffer[5];
    let transparency = buffer[6];
    Ok(PaletteDefinitionSegment {
        palette_id,
        palette_version_number,
        palette_entry_id,
        luminance,
        color_difference_red,
        color_difference_blue,
        transparency,
    })
}

#[derive(Debug)]
pub struct ObjectDefinitionSegment {
    object_id: u16,
    object_version_number: u8,
    last_in_sequence_flag: u8,
    object_data_lenght: u24,
    width: u16,
    height: u16,
    object_data: Vec<u8>, // ????
}

pub fn read_ods(file: &mut File) -> Result<ObjectDefinitionSegment, Error> {
    const ODS_HEADER: usize = 2 + 1 + 1 + 3 + 2 + 2; //size_of::<PaletteDefinitionSegment>();
    let mut buffer = [0u8; ODS_HEADER];
    let read_count = file.read(&mut buffer)?;
    if read_count < ODS_HEADER {
        return Err(String::from("Can't read all Object Definition Segment").into());
    }
    let object_id = u16::from_be_bytes(buffer[0..2].try_into().unwrap());
    let object_version_number = buffer[2];
    let last_in_sequence_flag = buffer[3];
    let object_data_lenght =
        u24::from(<&[u8] as TryInto<[u8; 3]>>::try_into(&buffer[4..7]).unwrap());
    let width = u16::from_be_bytes(buffer[7..9].try_into().unwrap());
    let height = u16::from_be_bytes(buffer[9..11].try_into().unwrap());
    //object_data: Vec<u8>, // ????

    let mut object_data = Vec::new();
    let data_size: usize = object_data_lenght.to_u32().try_into().unwrap();
    object_data.resize(data_size, 0);
    let read_count = file.read(object_data.as_mut_slice())?;
    if read_count < object_data.len() {
        return Err(String::from("Can't read all Object Data").into());
    }

    Ok(ObjectDefinitionSegment {
        object_id,
        object_version_number,
        last_in_sequence_flag,
        object_data_lenght,
        width,
        height,
        object_data,
    })
}
