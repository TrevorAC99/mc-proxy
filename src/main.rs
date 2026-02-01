use std::cmp::min;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::thread;

mod io_extensions;
mod varint;

use io_extensions::{ReadByte};
use crate::varint::{ReadVarInt, VarInt};


fn main() {
    println!("Hello, world!");
    let listener = std::net::TcpListener::bind("127.0.0.1:25565").unwrap();

    loop {
        let (client_stream, addr) = listener.accept().unwrap();
        println!("Connection from {}", addr);
        thread::spawn(move || {
            let result = handle_connection(client_stream);
            if let Err(e) = result {
                eprintln!("{} - {}", e.kind(), e);
            }
        });
    }
}

fn handle_connection(mut client_stream: TcpStream) -> std::io::Result<()> {
    let packet = read_uncompressed_packet(&mut client_stream)?;

    let handshake = parse_handshake(&packet.data)
        .map_err(|_| std::io::Error::new(ErrorKind::Other, "Failed to parse handshake"))?;
    println!("Protocol Version: {}", handshake.protocol_version);
    println!("Server Address  : {}", handshake.server_address);
    println!("Port            : {}", handshake.port);
    println!("Intent          : {}", handshake.intent);

    let server_address =
        lookup_destination(&handshake.server_address, handshake.port).ok_or_else(|| {
            eprintln!(
                "No proxy configured for {}:{}",
                handshake.server_address, handshake.port
            );
            std::io::Error::new(ErrorKind::Other, "Failed to parse handshake")
        })?;

    let mut server_stream = TcpStream::connect(&server_address).map_err(|err| {
        eprintln!("Failed to connect to {}: {}", &server_address, &err);
        err
    })?;

    // Forward the handshake packet
    packet.write_to(&mut server_stream)?;

    // Perhaps not the most efficient considering each connection uses three
    // threads, but until I switch to an async implementation this seems better
    // than switching the streams to non-blocking mode and running a single
    // thread in a hot loop. This way should at least let each thread yield
    // while waiting for io to come in on the file descriptor.
    let c_to_s = {
        let client_stream = client_stream.try_clone()?;
        let server_stream = server_stream.try_clone()?;
        thread::spawn(|| forward_stream(client_stream, server_stream))
    };

    let s_to_c = {
        let client_stream = client_stream.try_clone()?;
        let server_stream = server_stream.try_clone()?;
        thread::spawn(|| forward_stream(server_stream, client_stream))
    };

    c_to_s.join().unwrap()?;
    s_to_c.join().unwrap()?;

    println!("Connection closed");
    Ok(())
}

const FORWARD_BUFFER_SIZE: usize = 1024 * 4;

fn forward_stream(mut reader: TcpStream, mut writer: TcpStream) -> std::io::Result<()> {
    let mut buf = [0; FORWARD_BUFFER_SIZE];
    loop {
        // Reads one byte before trying to fill the entire buffer so that
        // the loop only runs when there is data to forward.
        reader
            .read_exact(&mut buf[..1])
            .map_err(|err| match err.kind() {
                ErrorKind::UnexpectedEof => {
                    std::io::Error::new(ErrorKind::ConnectionAborted, "connection closed")
                }
                _ => err,
            })?;
        let count = reader.read(&mut buf[1..])? + 1;
        println!("{} bytes read", count);
        writer.write_all(&buf[..count])?;
    }
}

/// TODO: Figure out why this doesn't work
fn _forward_packets(mut reader: TcpStream, mut writer: TcpStream) -> std::io::Result<()> {
    let mut len_buf = Vec::with_capacity(3);
    let mut buf = vec![0; FORWARD_BUFFER_SIZE];
    loop {
        len_buf.clear();
        let mut shift = 0;
        let mut length = 0;
        loop {
            let byte = reader.read_byte()?;
            length |= ((byte & SEGMENT_BITS) as usize) << shift;
            len_buf.push(byte);
            shift += 7;
            if (byte & CONTINUE_BIT) == 0 {
                break;
            }
            if shift >= i32::BITS {
                return Err(std::io::Error::new(
                    ErrorKind::Other,
                    "Invalid varint",
                ));
            }
        };

        print!("[{}] Packet length: {} -", reader.peer_addr()?, length);
        len_buf.iter().for_each(|byte| { print!(" {:08b} ", byte); });
        println!();
        writer.write_all(&len_buf)?;
        while length > 0 {
            let chunk_size = min(FORWARD_BUFFER_SIZE, length);
            println!("[{}] Chunk size: {}", reader.peer_addr()?, chunk_size);

            reader.read_exact(&mut buf[..chunk_size])?;
            writer.write_all(&buf[..chunk_size])?;

            length -= chunk_size;
        }
    }
}

fn lookup_destination(server_address: &str, port: u16) -> Option<String> {
    if port == 25565 && server_address.eq_ignore_ascii_case("localhost") {
        println!("Proxy found for {server_address}:{port}");
        Some("127.0.0.1:25566".to_string())
    } else {
        None
    }
}

const SEGMENT_BITS: u8 = 0b0111_1111;
const CONTINUE_BIT: u8 = 0b1000_0000;

fn read_string(bytes: &[u8], pos: &mut usize) -> Result<String, ()> {
    let length = read_varint(bytes, pos)? as usize;
    let string = String::from_utf8_lossy(&bytes[(*pos)..(*pos + length)]);
    *pos += length;
    Ok(String::from(string))
}

fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<i32, ()> {
    let mut value = 0;
    let mut shift = 0;
    loop {
        let byte = bytes[*pos];
        *pos += 1;
        value |= ((byte & SEGMENT_BITS) as i32) << shift;
        if (byte & CONTINUE_BIT) == 0 {
            return Ok(value);
        }
        shift += SEGMENT_BITS.count_ones();
        if shift >= i32::BITS {
            return Err(());
        }
    }
}

fn read_u16(bytes: &[u8], pos: &mut usize) -> u16 {
    *pos += 2;
    u16::from_be_bytes(bytes[*pos - 2..*pos].try_into().unwrap())
}

fn read_uncompressed_packet(stream: &mut TcpStream) -> std::io::Result<UncompressedPacket> {
    let length = stream.read_varint()?;
    if length.value() == 0 {
        eprintln!("[{}] Empty packet", stream.peer_addr()?);
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Zero length packet",
        ));
    }

    let packet_id = stream.read_varint()?;

    let mut data = vec![0u8; length.value() as usize - packet_id.len()];
    stream.read_exact(&mut data)?;

    Ok(UncompressedPacket {
        length: length,
        packet_id: packet_id,
        data,
    })
}

struct UncompressedPacket {
    length: VarInt,
    packet_id: VarInt,
    data: Vec<u8>,
}

impl UncompressedPacket {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        _ = self.length;
        _ = self.packet_id;
        writer.write_all(&self.length.bytes())?;
        writer.write_all(&self.packet_id.bytes())?;
        writer.write_all(&self.data)
    }
}

struct Handshake {
    protocol_version: i32,
    server_address: String,
    port: u16,
    intent: i32,
}

fn parse_handshake(bytes: &[u8]) -> Result<Handshake, ()> {
    let mut pos = 0;
    Ok(Handshake {
        protocol_version: read_varint(bytes, &mut pos)?,
        server_address: read_string(bytes, &mut pos)?,
        port: read_u16(bytes, &mut pos),
        intent: read_varint(bytes, &mut pos)?,
    })
}
