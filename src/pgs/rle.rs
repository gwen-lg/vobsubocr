use std::io::Cursor;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

pub fn decode_rle<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    let data = data.as_ref();
    let data_len = data.len() as u64;
    let mut c = Cursor::new(data);
    let mut output = Vec::with_capacity(data.len());

    loop {
        if c.position() >= data_len {
            break;
        }

        // check first byte color
        match c.read_u8().unwrap() {
            0x00 => {}
            _ => {
                output.push(1);
                continue;
            }
        }
        // check second byte for length
        let info = match c.read_u8().unwrap() {
            0x00 => {
                // output.push(2);
                continue;
            }
            x => x,
        };
        let is_color = is_color(info);
        let big_len = is_long(info);
        let len_u8 = info & 0b0011_1111;
        assert_eq!(len_u8 >> 6, 0);

        // println!("big len: {}", big_len);
        // println!("high len: {}", len_u8);

        let len = if big_len {
            let len2_u8 = c.read_u8().unwrap();
            // println!("low len: {}", len2_u8);
            let buf = [len_u8, len2_u8];
            BigEndian::read_u16(&buf)
        } else {
            len_u8 as u16
        };

        let color = if is_color {
            c.read_u8().unwrap()
        } else {
            // use preferred color
            0
        };

        // println!("{} colored {}", len, color);
        for x in 0..len {
            output.push(color);
        }
    }
    output
}

fn is_color(byte: u8) -> bool {
    byte >> 7 == 1
}

fn is_long(byte: u8) -> bool {
    (byte >> 6) & 0b1 == 1
}
