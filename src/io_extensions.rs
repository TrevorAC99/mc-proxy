use std::io::Read;

pub trait ReadByte {
    fn read_byte(&mut self) -> std::io::Result<u8>;
}

impl<R: Read> ReadByte for R {
    fn read_byte(&mut self) -> std::io::Result<u8> {
        let mut byte = [0u8];
        self.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}
