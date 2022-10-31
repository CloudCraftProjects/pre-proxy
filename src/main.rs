extern crate core;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use anyhow::Result;
use log::{error, info, warn};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use crate::buffer::{Buf, s_write_var_u32};
use crate::config::Config;

mod buffer;
mod protocol;

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
    info!("Hello, world!");

    let config_path = args.get(1).unwrap_or(&String::from("config.yml"));
    let config = Config::load_or_init(Path::new(config_path));

    let bind_addr = config.get_bind_addr();
    let redir_addr = config.get_redir_addr();

    let mut listener = TcpListener::bind(bind_addr).await.unwrap();

    loop {
        let client = accept_client(&mut listener).await;
        if let Err(err) = client {
            error!("Failed to accept client: {}", err);
            continue;
        }

        let unwrapped_client = client.unwrap();
        let stream = unwrapped_client.0;
        let address = unwrapped_client.1;

        tokio::spawn(async move {
            let result = handle_client(stream, redir_addr, address).await;

            if let Err(err) = result {
                error!("{}: An error occurred: {}", address, err);
            }
        });
    }
}

async fn accept_client(stream: &mut TcpListener) -> Result<(TcpStream, SocketAddr)> {
    let client = stream.accept().await?;
    client.0.set_nodelay(true)?;
    return Ok(client);
}

async fn handle_client(stream: TcpStream, connect: SocketAddr, address: SocketAddr) -> Result<()> {
    let mut server = TcpStream::connect(connect).await?;
    server.set_nodelay(true)?;

    let mut info_buf = Buf::new();

    // Identifier for this pre-proxy
    info_buf.write_u32(69_1337_42u32);

    match address.ip() {
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
    info_buf.write_u16(address.port());

    s_write_var_u32(&mut server, info_buf.get_writer_index()).await;
    server.write(info_buf.read_remaining_bytes()).await?;

    let (mut client_reader, mut client_writer) = tokio::io::split(stream);
    let (mut server_reader, mut server_writer) = tokio::io::split(server);

    tokio::spawn(async move {
        let result = tokio::io::copy(&mut client_reader, &mut server_writer).await;
        if let Some(err) = result.err() {
            warn!("[{}] Error in client-to-server bridge: {}", address, err);
        }
    });

    let result = tokio::io::copy(&mut server_reader, &mut client_writer).await;
    if let Some(err) = result.err() {
        warn!("[{}] Error in server-to-client bridge: {}", address, err);
    }

    return Ok(());
}
