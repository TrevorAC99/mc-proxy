use crate::io_extensions::ReadByte;
use std::io::Read;

const SEGMENT_BITS: u8 = 0b0111_1111;
const CONTINUE_BIT: u8 = 0b1000_0000;

pub struct VarInt {
    bytes: [u8; 5],
}

impl VarInt {
    pub fn new(bytes: [u8; 5]) -> VarInt {
        VarInt { bytes }
    }
    pub fn value(&self) -> i32 {
        let mut value: i32 = 0;
        let mut shift = 0;
        for byte in self.bytes {
            value |= ((byte & SEGMENT_BITS) as i32) << shift;
            shift += 7;
            if (byte & CONTINUE_BIT) == 0 {
                break;
            }
        }
        value
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for byte in self.bytes {
            len += 1;
            if (byte & CONTINUE_BIT) == 0 {
                break;
            }
        }
        len
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }
}

pub trait ReadVarInt {
    fn read_varint(&mut self) -> std::io::Result<VarInt>;
}

const VARINT_MAX_BYTES: usize = 5;

impl<R: Read> ReadVarInt for R {
    fn read_varint(&mut self) -> std::io::Result<VarInt> {
        let mut bytes: [u8; 5] = [0; 5];
        let mut idx = 0;
        loop {
            let byte = self.read_byte()?;
            bytes[idx] = byte;
            if (byte & CONTINUE_BIT) == 0 {
                return Ok(VarInt { bytes });
            }
            idx += 1;
            if idx >= VARINT_MAX_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Invalid varint",
                ));
            }
        }
    }
}

pub fn read_varint(input: &[u8], pos: &mut usize) -> Result<VarInt, ()> {
    let mut bytes = [0u8; 5];
    let mut idx = 0;
    loop {
        let byte = input[*pos];
        bytes[idx] = byte;
        *pos += 1;
        if (byte & CONTINUE_BIT) == 0 {
            let varint = VarInt::new(bytes);
            return Ok(varint);
        }
        idx += 1;
        if idx > 4 {
            return Err(());
        }
    }
}
