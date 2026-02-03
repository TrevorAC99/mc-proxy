use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(version, about = "Reverse proxy for Minecraft servers", long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "127.0.0.1:25565")]
    pub listen: SocketAddr,
}
