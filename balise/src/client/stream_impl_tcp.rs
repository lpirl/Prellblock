use crate::{Error, Address};
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub type StreamImpl = TcpStream;

impl<'a> super::StreamGuard<'a> {
    pub fn tcp_stream(&self) -> &TcpStream {
        self.stream.as_ref().unwrap()
    }
}

pub async fn connect(addr: &Address) -> Result<StreamImpl, Error> {
    let stream = TcpStream::connect((addr.host.to_string(), addr.port)).await?;
    Ok(stream)
}
