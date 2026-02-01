use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;

mod io_extensions;
mod packet;
mod varint;

use crate::packet::{ReadPacket, WritePacket};
use crate::varint::read_varint;

fn to_io_err(msg: &'static str) -> std::io::Error {
    std::io::Error::new(ErrorKind::Other, msg)
}

fn main() {
    println!("Starting proxy...");
    let bind_addr: SocketAddr = "127.0.0.1:25565".parse().unwrap();
    let listener = std::net::TcpListener::bind(bind_addr).unwrap();

    loop {
        let (client_stream, addr) = listener.accept().unwrap();
        println!("Connection from {}", addr);
        thread::spawn(move || {
            let result = handle_connection(client_stream, bind_addr);
            if let Err(e) = result {
                eprintln!("{} - {}", e.kind(), e);
            }
        });
    }
}

fn handle_connection(mut client_stream: TcpStream, self_addr: SocketAddr) -> std::io::Result<()> {
    let packet = client_stream.read_packet()?;

    let data = packet.data(false).map_err(to_io_err)?;

    let handshake = parse_handshake(data).map_err(|_| to_io_err("Failed to parse handshake"))?;
    println!("Protocol Version: {}", handshake.protocol_version);
    println!("Server Address  : {}", handshake.server_address);
    println!("Port            : {}", handshake.port);
    println!("Intent          : {}", handshake.intent);

    let server_addr =
        lookup_destination(&handshake.server_address, handshake.port).ok_or_else(|| {
            eprintln!(
                "No proxy configured for {}:{}",
                handshake.server_address, handshake.port
            );
            std::io::Error::new(ErrorKind::Other, "Failed to parse handshake")
        })?;

    if server_addr == self_addr {
        eprintln!(
            "Proxy for {}:{} will cause an infinite loop!",
            handshake.server_address, handshake.port
        );
        return Err(std::io::Error::new(
            ErrorKind::Other,
            "Configured proxy will cause an infinite loop!",
        ));
    }

    let mut server_stream = TcpStream::connect(&server_addr).map_err(|err| {
        eprintln!("Failed to connect to {}: {}", &server_addr, &err);
        err
    })?;

    // Forward the handshake packet
    server_stream.write_packet(packet)?;

    // Perhaps not the most efficient considering each connection uses two
    // threads, but until I switch to an async implementation this seems better
    // than switching the streams to non-blocking mode and running a single
    // thread in a hot loop. This way should at least let each thread yield
    // while waiting for io to come in on the file descriptor.
    let c_to_s = {
        let client_stream = client_stream.try_clone()?;
        let server_stream = server_stream.try_clone()?;
        thread::spawn(|| forward_stream(client_stream, server_stream))
    };

    forward_stream(server_stream, client_stream)?;

    c_to_s.join().unwrap()?;

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
        writer.write_all(&buf[..count])?;
    }
}

fn lookup_destination(server_address: &str, port: u16) -> Option<SocketAddr> {
    // TODO: Make this configurable
    if port == 25565 && server_address.eq_ignore_ascii_case("localhost") {
        println!("Proxy found for {server_address}:{port}");
        Some("127.0.0.1:25566".parse().unwrap())
    } else {
        None
    }
}

fn read_string(bytes: &[u8], pos: &mut usize) -> Result<String, ()> {
    let length = read_varint(bytes, pos)?.value() as usize;
    let string = String::from_utf8_lossy(&bytes[(*pos)..(*pos + length)]);
    *pos += length;
    Ok(String::from(string))
}

fn read_u16(bytes: &[u8], pos: &mut usize) -> Result<u16, ()> {
    *pos += 2;
    if bytes.len() < *pos {
        return Err(());
    }
    Ok(u16::from_be_bytes(
        bytes[*pos - 2..*pos].try_into().unwrap(),
    ))
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
        protocol_version: read_varint(bytes, &mut pos)?.value(),
        server_address: read_string(bytes, &mut pos)?,
        port: read_u16(bytes, &mut pos)?,
        intent: read_varint(bytes, &mut pos)?.value(),
    })
}
