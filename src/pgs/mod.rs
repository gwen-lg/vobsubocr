mod segment;
mod u24;

use core::fmt;
use snafu::Snafu;
use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    io::{self, BufReader, Read},
};

use crate::{
    opt::Opt,
    pgs::segment::{read_ods, read_pcs, read_pds, read_wds, SegmentType},
};

use self::segment::read_header;

// https://blog.thescorpius.com/index.php/2017/07/15/presentation-graphic-stream-sup-files-bluray-subtitle-format/
//TODO: extract info avoir partition with error, and faile operation with collect when error in iterator
//TODO: check terresac setup : https://github.com/ratoaq2/pgsrip/blob/master/pgsrip/pgs.py#L73

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

//const READ_SIZE: usize = 1024 * 1024;
const BUFFER_CAPACITY: usize = 1024 * 1024;

// pub struct BufferMngr {
//     buffer: Vec<u8>,
//     buf_cursor: usize,
//     //reach_eof: bool,
// }

// impl BufferMngr {
//     pub fn new() -> Self {
//         Self {
//             buffer: Vec::new(), //with_capacity(BUFFER_SIZE),
//             buf_cursor: 0,
//             //reach_eof: false,
//         }
//     }
//     pub fn read_from_file(&mut self, file: &mut File) -> Result<(), std::io::Error> {
//         let read_count = file.read_to_end(&mut self.buffer)?;
//         //self.reach_eof = read_count < READ_SIZE; //TODO manage
//         self.buf_cursor = 0;
//         Ok(())
//     }
//     pub fn take_slice(&mut self, count: usize) -> &[u8] {
//         let (_, right) = self.buffer.split_at(self.buf_cursor);
//         let (left, _) = right.split_at(count);
//         self.buf_cursor = self.buf_cursor + count;
//         left
//     }
// }

pub type Result<T, E = crate::pgs::Error> = std::result::Result<T, E>;

pub fn run(opt: &Opt) -> Result<()> {
    //let mut buf_mngr = BufferMngr::new();
    let mut file = File::open(opt.input.clone())?;
    //buf_mngr.read_from_file(&mut file)?;
    let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, file);
    while let Some(segment_header) = Some(read_header(&mut reader)?) {
        println!("Segment : {segment_header}");
        match segment_header.sg_type() {
            SegmentType::Pcs => {
                let pcs = read_pcs(&mut reader)?;
                println!("PCS: {pcs:?}");
            }
            SegmentType::Wds => {
                let wds = read_wds(&mut reader)?;
                println!("WDS: {wds:?}");
            }
            SegmentType::Pds => {
                let pds = read_pds(&mut reader, segment_header.size().into())?;
                println!("PDS: {pds:?}");
            }
            SegmentType::Ods => {
                let ods = read_ods(&mut reader, segment_header.size().into())?;
                println!("ODS: {ods:?}");
            }
            SegmentType::End => {
                println!("END");
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

#[derive(Debug)]
struct ObjectCroppingInfo {
    object_cropping_horizontal_position: u16, // X offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_vertical_position: u16, // Y offset from the top left pixel of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_width: u16, // Width of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
    object_cropping_height_position: u16, // Heightl of the cropped object in the screen. Only used when the Object Cropped Flag is set to 0x40.
}

#[derive(Debug)]
struct WindowInformationObject {
    object_id: u16,          // ID of the ODS segment that defines the image to be shown
    window_id: u8, // Id of the WDS segment to which the image is allocated in the PCS. Up to two images may be assigned to one window
    object_cropped_flag: u8, // 0x40: Force display of the cropped image object, 0x00: Off
    object_horizontal_position: u16, // X offset from the top left pixel of the image on the screen
    object_vertical_position: u16, // Y offset from the top left pixel of the image on the screen
    object_cropping_info: Option<ObjectCroppingInfo>,
}
fn read_window_info(reader: &mut BufReader<File>) -> Result<WindowInformationObject, Error> {
    const WIN_INFO_LEN: usize = 2 + 1 + 1 + 2 + 2;
    let mut win_info_buf = [0; WIN_INFO_LEN];
    let read = reader.read(&mut win_info_buf)?;
    if read < WIN_INFO_LEN {
        return Err(String::from("Can't read engouth data").into());
    }
    let object_id = u16::from_be_bytes(win_info_buf[0..2].try_into().unwrap());
    let window_id = win_info_buf[2];
    let object_cropped_flag = win_info_buf[3];
    if object_cropped_flag != 0x00 && object_cropped_flag != 0x40 {
        //	Indicates if this PCS describes a Palette only Display Update. Allowed values are: 0x00: False | 0x80: True
        return Err(String::from("TODO object_cropped_flag").into());
    }
    let object_horizontal_position = u16::from_be_bytes(win_info_buf[4..6].try_into().unwrap());
    let object_vertical_position = u16::from_be_bytes(win_info_buf[6..8].try_into().unwrap());

    let object_cropping_info = if object_cropped_flag == 0x40 {
        const CROPPING_INFO_LEN: usize = 2 + 2 + 2 + 2;
        let mut cropping_info_buf = [0; CROPPING_INFO_LEN];
        let read = reader.read(&mut cropping_info_buf)?;
        if read < CROPPING_INFO_LEN {
            return Err(String::from("Can't read engouth data").into());
        }

        let object_cropping_horizontal_position =
            u16::from_be_bytes(cropping_info_buf[0..2].try_into().unwrap());
        let object_cropping_vertical_position =
            u16::from_be_bytes(cropping_info_buf[2..4].try_into().unwrap());
        let object_cropping_width = u16::from_be_bytes(cropping_info_buf[4..6].try_into().unwrap());
        let object_cropping_height_position =
            u16::from_be_bytes(cropping_info_buf[6..8].try_into().unwrap());
        Some(ObjectCroppingInfo {
            object_cropping_horizontal_position,
            object_cropping_vertical_position,
            object_cropping_width,
            object_cropping_height_position,
        })
    } else {
        None
    };
    Ok(WindowInformationObject {
        object_id,
        window_id,
        object_cropped_flag,
        object_horizontal_position,
        object_vertical_position,
        object_cropping_info,
    })
}
