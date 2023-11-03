extern crate core;

use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::FutureExt;
use log::{error, info};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

use crate::buffer::Buf;
use crate::config::Config;
use crate::protocol::{close_connection, read_packet, send_packet, validate_handshake};

mod buffer;
mod protocol;
mod config;

const MAX_HANDSHAKE_LENGTH: u32 = 0
    + 5 // protocol version (var u32)
    + 1 // hostname length (var u32)
    + 255 // hostname (string), can theoretically be non-ascii, but that doesn't happen
    + 2 // port (u16)
    + 1 // next status (var u32), can only be 1 or 2, so length is 1
;

const BUF_SIZE: usize = 1024;

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
            read_handshake(&config, stream, &addr).await.unwrap();
        });
    }
}

async fn accept_client(stream: &mut TcpListener) -> Result<(TcpStream, SocketAddr)> {
    let client = stream.accept().await?;
    client.0.set_nodelay(true)?;
    return Ok(client);
}

// copied from https://github.com/mqudsi/tcpproxy/blob/99f64a3b3d7509ca77fbfa5e9cade48d72202166/src/main.rs (MIT license),
// as tokio's copy methods don't close connections properly

// Two instances of this function are spawned for each half of the connection: client-to-server,
// server-to-client. We can't use tokio::io::copy() instead (no matter how convenient it might
// be) because it doesn't give us a way to correlate the lifetimes of the two tcp read/write
// loops: even after the client disconnects, tokio would keep the upstream connection to the
// server alive until the connection's max client idle timeout is reached.
async fn copy_with_abort<R, W>(
    read: &mut R,
    write: &mut W,
    mut abort: broadcast::Receiver<()>,
) -> tokio::io::Result<usize>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
{
    let mut copied = 0;
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let bytes_read;
        tokio::select! {
                biased;

                result = read.read(&mut buf) => {
                    use std::io::ErrorKind::{ConnectionReset, ConnectionAborted};
                    bytes_read = result.or_else(|e| match e.kind() {
                        // Consider these to be part of the proxy life, not errors
                        ConnectionReset | ConnectionAborted => Ok(0),
                        _ => Err(e)
                    })?;
                },
                _ = abort.recv() => {
                    break;
                }
            }

        if bytes_read == 0 {
            break;
        }

        // While we ignore some read errors above, any error writing data we've already read to
        // the other side is always treated as exceptional.
        write.write_all(&buf[0..bytes_read]).await?;
        copied += bytes_read;
    }

    Ok(copied)
}

async fn start_proxy(mut stream: TcpStream, mut server: TcpStream) -> Option<()> {
    let (mut client_reader, mut client_writer) = stream.split();
    let (mut server_reader, mut server_writer) = server.split();

    let (cancel, _) = broadcast::channel::<()>(1);
    let (remote_copied, client_copied) = tokio::join! {
                copy_with_abort(&mut server_reader, &mut client_writer, cancel.subscribe())
                    .then(|r| { let _ = cancel.send(()); async { r } }),
                copy_with_abort(&mut client_reader, &mut server_writer, cancel.subscribe())
                    .then(|r| { let _ = cancel.send(()); async { r } }),
            };

    match client_copied {
        Ok(_) => {}
        Err(err) => {
            error!("Error writing bytes from proxy client to upstream server: {}", err);
        }
    };

    match remote_copied {
        Ok(_) => {}
        Err(err) => {
            error!("Error writing from upstream server to proxy client: {}", err);
        }
    };

    return Some(());
}

async fn read_handshake(config: &Config, mut stream: TcpStream, addr: &SocketAddr) -> Option<()> {
    let handshake_result = read_packet(&mut stream, MAX_HANDSHAKE_LENGTH).await;
    return match handshake_result {
        Ok(handshake) => {
            if let Some(validated_handshake) = validate_handshake(&mut handshake.1.copy(), addr) {
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
            connect_client(config, stream, addr, handshake).await
        }
        Err(err) => {
            error!("Error reading handshake from client: {}", err);
            Some(())
        }
    }
}

async fn connect_client(config: &Config, stream: TcpStream, addr: &SocketAddr, handshake: (u32, Buf)) -> Option<()> {
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

    return start_proxy(stream, server).await;
}
