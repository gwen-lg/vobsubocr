mod segment;
mod u24;

use core::fmt;
use snafu::Snafu;
use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    io::{self, Read},
};

use crate::{
    opt::Opt,
    pgs::segment::{read_ods, read_pcs, read_pds, read_wds, SegmentType},
};

use self::segment::read_header;

// https://blog.thescorpius.com/index.php/2017/07/15/presentation-graphic-stream-sup-files-bluray-subtitle-format/

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not build tesseract thread pool: {}", source))]
    IoError { source: io::Error },

    #[snafu(display("Parse: {}", value))]
    String { value: String }, //TODO

    #[snafu(display("EndOfFile found"))]
    EndOfFile,
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IoError { source: value }
    }
}
impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::String { value }
    }
}

pub type Result<T, E = crate::pgs::Error> = std::result::Result<T, E>;

pub fn run(opt: &Opt) -> Result<()> {
    let mut file = File::open(opt.input.clone())?;
    while let Some(segment_header) = Some(read_header(&mut file)?) {
        println!("Segment : {segment_header}");
        match segment_header.sg_type() {
            SegmentType::Pcs => {
                let pcs = read_pcs(&mut file)?;
                println!("PCS: {pcs:?}");
            }
            SegmentType::Wds => {
                let wds = read_wds(&mut file)?;
                println!("WDS: {wds:?}");
            }
            SegmentType::Pds => {
                let pds = read_pds(&mut file)?;
                println!("PDS: {pds:?}");
            }
            SegmentType::Ods => {
                let ods = read_ods(&mut file)?;
                println!("ODS: {ods:?}");
            }
            SegmentType::End => {
                println!("END: nothing to read");
            }
        }
    }
    Ok(())
}

#[repr(u8)]
enum CompositionState {
    Normal = 0x00,
    AcquisitionPoint = 0x40,
    EpochStart = 0x80,
}
impl TryFrom<u8> for CompositionState {
    type Error = Error;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x00 => Ok(CompositionState::Normal),
            0x40 => Ok(CompositionState::AcquisitionPoint),
            0x80 => Ok(CompositionState::EpochStart),
            _ => Err(Error::String {
                value: String::from("invalid value for CompositionState"), //TODO: better use Snafu
            }),
        }
    }
}
impl fmt::Debug for CompositionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            CompositionState::Normal => "Normal",
            CompositionState::AcquisitionPoint => "AcquisitionPoint",
            CompositionState::EpochStart => "EpochStart",
        };
        write!(f, "{str}")
    }
}

struct WindowInformationObject {
    object_id: u16,          // ID of the ODS segment that defines the image to be shown
    window_id: u8, // Id of the WDS segment to which the image is allocated in the PCS. Up to two images may be assigned to one window
    object_cropped_flag: u8, // 0x40: Force display of the cropped image object, 0x00: Off
    object_horizontal_position: u16, // X offset from the top left pixel of the image on the screen
    object_vertical_position: u16, // Y offset from the top left pixel of the image on the screen
    object_cropping_horizontal_position: u16, // X offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_vertical_position: u16, // Y offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_width: u16, // Width of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_height_position: u16, // Heightl of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
}
fn read_window_info(file: &mut File) -> Result<WindowInformationObject, Error> {
    const WIN_INFO_LEN: usize = 2 + 1 + 1 + 2 + 2 + 2 + 2 + 2 + 2;
    let mut buffer = [0u8; WIN_INFO_LEN];
    let read_count = file.read(&mut buffer)?;
    if read_count < WIN_INFO_LEN {
        return Err(
            String::from("Invalid file, can't read entire struct WindowInformationObject").into(),
        );
    }
    let object_id = u16::from_be_bytes(buffer[0..2].try_into().unwrap());
    let window_id = buffer[2];
    let object_cropped_flag = buffer[3];
    if object_cropped_flag != 0x00 && object_cropped_flag != 0x40 {
        //	Indicates if this PCS describes a Palette only Display Update. Allowed values are: 0x00: False | 0x80: True
        return Err(String::from("TODO object_cropped_flag").into());
    }
    let object_horizontal_position = u16::from_be_bytes(buffer[4..6].try_into().unwrap());
    let object_vertical_position = u16::from_be_bytes(buffer[6..8].try_into().unwrap());
    let object_cropping_horizontal_position = u16::from_be_bytes(buffer[8..10].try_into().unwrap());
    let object_cropping_vertical_position = u16::from_be_bytes(buffer[10..12].try_into().unwrap());
    let object_cropping_width = u16::from_be_bytes(buffer[12..14].try_into().unwrap());
    let object_cropping_height_position = u16::from_be_bytes(buffer[14..16].try_into().unwrap());
    Ok(WindowInformationObject {
        object_id,
        window_id,
        object_cropped_flag,
        object_horizontal_position,
        object_vertical_position,
        object_cropping_horizontal_position,
        object_cropping_vertical_position,
        object_cropping_width,
        object_cropping_height_position,
    })
}
