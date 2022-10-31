extern crate core;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use log::{error, info, warn};
use tokio::net::{TcpListener, TcpStream};

use crate::buffer::Buf;
use crate::config::Config;
use crate::protocol::{close_connection, read_packet, send_packet, validate_handshake};

mod buffer;
mod protocol;
mod config;

static MAX_HANDSHAKE_LENGTH: u32 = 0
    + 5 // protocol version (var u32)
    + 1 // hostname length (var u32)
    + 255 // hostname (string), can theoretically be non-ascii, but that doesn't happen
    + 2 // port (u16)
    + 1 // next status (var u32), can only be 1 or 2, so length is 1
;

#[tokio::main]
async fn main() {
    // https://github.com/DoubleCheck0001/rust-minecraft-proxy/blob/47923992632b4990e9149b663817cbef4f01e388/src/main.rs
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Heavily inspired by https://github.com/Eoghanmc22/rust-mc-bot/blob/b77382357cb0995446a6973d786085aa3abea029/src/main.rs,
    // because I don't have a good understanding of rust's syntax yet :(
    let args: Vec<String> = env::args().collect();

    let def_config_path = String::from("config.yml");
    let config_path = args.get(1).unwrap_or(&def_config_path);

    let config = Config::load_or_init(Path::new(config_path));
    let mut listener = TcpListener::bind(config.get_bind_addr()).await.unwrap();

    info!("listening on {}, redirecting to {}", config.get_bind_addr(), config.get_connect_addr());
    let config = Arc::new(config);

    loop {
        let client = accept_client(&mut listener).await;
        if let Err(err) = client {
            error!("failed to accept client: {}", err);
            continue;
        }

        let (stream, addr) = client.unwrap();
        let config = Arc::clone(&config);

        tokio::spawn(async move {
            handle_client(&config, stream, &addr).await.unwrap();
        });
    }
}

async fn accept_client(stream: &mut TcpListener) -> Result<(TcpStream, SocketAddr)> {
    let client = stream.accept().await?;
    client.0.set_nodelay(true)?;
    return Ok(client);
}

async fn handle_client(config: &Config, mut stream: TcpStream, addr: &SocketAddr) -> Option<()> {
    let handshake = read_packet(&mut stream, MAX_HANDSHAKE_LENGTH).await.unwrap();
    if let Some(validated_handshake) = validate_handshake(&mut handshake.1.copy(), addr) {
        info!("handshake version: {}, vhost: {}:{}, next state: {}", validated_handshake.0, validated_handshake.1, validated_handshake.2, if validated_handshake.3 == 2 { "login" }  else {"status" });
        let host = validated_handshake.1.as_str();
        if !config.check_host(host) {
            error!("invalid hostname specified in packet: {}", host);
            close_connection(&mut stream).await;
            return Some(());
        }
    } else {
        close_connection(&mut stream).await;
        return Some(());
    }

    let mut server = TcpStream::connect(config.get_connect_addr()).await.unwrap();
    server.set_nodelay(true).unwrap();

    let mut info_buf = Buf::new();
    match addr.ip() {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            info_buf.write_var_u32(octets.len() as u32);
            info_buf.write_bytes(octets.as_slice());
        }
        IpAddr::V6(ip) => {
            let octets = ip.octets();
            info_buf.write_var_u32(octets.len() as u32);
            info_buf.write_bytes(octets.as_slice());
        }
    }
    info_buf.write_u16(addr.port());

    send_packet(&mut server, 69_1337_42u32, info_buf).await; // write connection info
    send_packet(&mut server, handshake.0, handshake.1).await; // write mc handshake

    let (mut client_reader, mut client_writer) = tokio::io::split(stream);
    let (mut server_reader, mut server_writer) = tokio::io::split(server);

    tokio::spawn(async move {
        let result = tokio::io::copy(&mut client_reader, &mut server_writer).await;
        if let Some(err) = result.err() {
            warn!("error in serverbound writer: {}", err);
        }
    });

    let result = tokio::io::copy(&mut server_reader, &mut client_writer).await;
    if let Some(err) = result.err() {
        warn!("Error in clientbound writer: {}", err);
    }

    return Some(());
}
