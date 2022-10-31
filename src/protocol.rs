use std::net::SocketAddr;

use anyhow::Result;
use log::error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::buffer::{Buf, get_var_u32_size, s_write_var_u32};

pub async fn close_connection(stream: &mut TcpStream) {
    stream.shutdown().await.expect("error while shutting down");
}

pub async fn send_packet(stream: &mut TcpStream, packet_id: u32, mut buf: Buf) {
    let buf_bytes = buf.read_remaining_bytes();
    let packet_length = buf_bytes.len() as u32 + get_var_u32_size(packet_id);

    s_write_var_u32(stream, packet_length).await;
    s_write_var_u32(stream, packet_id).await;

    stream.write(buf_bytes).await.expect("failed to write packet bytes");
}

pub async fn read_packet(stream: &mut TcpStream, max_length: u32) -> Result<(u32/*packet id*/, Buf/*packet buf*/)> {
    let mut packet_buf = Buf::read(stream, max_length).await?;
    let packet_id = packet_buf.read_var_u32().0;
    return Ok((packet_id, packet_buf));
}

pub fn validate_handshake(handshake_buf: &mut Buf, addr: &SocketAddr) -> Option<(u32, String, u16, u32)> {
    // Reading this already acts as a kind of "validation"
    let protocol_version = handshake_buf.read_var_u32().0;
    let hostname = String::from(handshake_buf.read_sized_string());
    let port = handshake_buf.read_u16();
    let next_status = handshake_buf.read_var_u32().0;

    if (protocol_version >> 30 /*snapshot protocol bit*/) == 1 {
        // snapshot version
    } else if protocol_version < 800 /*will hopefully take mc a while to get to this version*/ && protocol_version > 0 /*versions less than 1 didn't exist*/ {
        // release version
    } else {
        // sus version, neither snapshot nor current release
        error!("{}: neither valid snapshot nor valid release version: {} (0x{:X})", addr,protocol_version, protocol_version);
        return None;
    }

    if next_status != 1 && next_status != 2 {
        error!("invalid next state specified: {}", next_status);
        return None;
    }

    return Some((protocol_version, hostname, port, next_status));
}
