use std::io::Read;
use crate::io_extensions::ReadByte;

pub struct VarInt {
    bytes: [u8; 5],
}

impl VarInt {
    pub fn value(&self) -> i32 {
        let mut value: i32 = 0;
        let mut shift = 0;
        for byte in self.bytes {
            value |= ((byte & 0b0111_1111) as i32) << shift;
            shift += 7;
            if (byte & 0b1000_0000) == 0 {
                break;
            }
        }
        value
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for (idx, byte) in self.bytes.iter().enumerate() {
            len += 1;
            if (byte & 0b1000_0000) == 0 {
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
    fn read_varint_limited(&mut self, max_bytes: usize) -> std::io::Result<VarInt>;
}

impl<R: Read> ReadVarInt for R {
    fn read_varint(&mut self) -> std::io::Result<VarInt> {
        self.read_varint_limited(5)
    }

    fn read_varint_limited(&mut self, max_bytes: usize) -> std::io::Result<VarInt> {
        let mut bytes: [u8; 5] = [0; 5];
        let mut idx = 0;
        loop {
            let byte = self.read_byte()?;
            bytes[idx] = byte;
            if (byte & 0b1000_0000) == 0 {
                let varint = VarInt {bytes};
                return Ok(VarInt {bytes});
            }
            idx += 1;
            if idx >= max_bytes {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid varint"));
            }
        }
    }
}
