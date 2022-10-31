use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub async fn close_connection(stream: &mut TcpStream) {
    stream.shutdown().await.expect("error while shutting down");
}
