use crate::read_varint;
use crate::result::Result;
use crate::varint::{ReadVarInt, VarInt};
use std::io::{Read, Write};

pub struct Packet {
    length: VarInt,
    bytes: Vec<u8>,
}

impl Packet {
    pub fn packet_id(&self, compressed: bool) -> Result<VarInt> {
        let mut pos = 0;
        if !compressed {
            read_varint(&self.bytes, &mut pos).map_err(|_| "Failed to read packet_id varint".into())
        } else {
            Err("Packet::packet_id() not yet implemented for compressed packets".into())
        }
    }

    pub fn data(&self, compressed: bool) -> Result<&[u8]> {
        let packet_id = self.packet_id(compressed)?;
        if !compressed {
            Ok(&self.bytes[packet_id.len()..])
        } else {
            Err("Packet::data() not yet implemented for compressed packets".into())
        }
    }
}

pub trait WritePacket {
    fn write_packet(self, packet: Packet) -> std::io::Result<()>;
}

impl<W: Write> WritePacket for &mut W {
    fn write_packet(self, packet: Packet) -> std::io::Result<()> {
        self.write_all(packet.length.bytes())?;
        self.write_all(&packet.bytes)
    }
}

pub trait ReadPacket {
    fn read_packet(self) -> std::io::Result<Packet>;
}

impl<R: Read> ReadPacket for &mut R {
    fn read_packet(self) -> std::io::Result<Packet> {
        let length = self.read_varint()?;
        let mut bytes = vec![0; length.value() as usize];
        self.read_exact(&mut bytes)?;
        Ok(Packet { length, bytes })
    }
}
