//! A server for communicating between RPUs.

use super::BoxError;
use serde::de::DeserializeOwned;
use std::{
    convert::TryInto,
    fmt::Debug,
    io::{self, Read, Write},
    marker::PhantomData,
    net::{SocketAddr, TcpListener, TcpStream},
};

/// A transparent response to a `Request`.
///
/// Use the `handle` method to create a matching response.
pub struct Response(pub(crate) serde_json::Value);

/// A Server (server) instance.
pub struct Server<T, H> {
    request_data: PhantomData<T>,
    handler: H,
}

impl<T, H> Clone for Server<T, H>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            request_data: PhantomData,
            handler: self.handler.clone(),
        }
    }
}

impl<T, H> Server<T, H>
where
    T: DeserializeOwned + Debug,
    H: Handler<T> + Clone,
{
    /// Create a new server instance.
    ///
    /// The `handler` needs to provide a `handle` callback script to handle requests on the server.
    #[must_use]
    pub fn new(handler: H) -> Self {
        Self {
            request_data: PhantomData,
            handler,
        }
    }

    /// The main server loop.
    pub fn serve(self, listener: &TcpListener) -> Result<(), BoxError>
    where
        T: Send + 'static,
        H: Send + 'static,
    {
        log::info!(
            "Server is now listening on Port {}",
            listener.local_addr()?.port()
        );
        for stream in listener.incoming() {
            // TODO: Is there a case where we should continue to listen for incoming streams?
            let stream = stream?;

            let clone_self = self.clone();

            // handle the client in a new thread
            std::thread::spawn(move || {
                let peer_addr = stream.peer_addr().unwrap();
                log::info!("Connected: {}", peer_addr);
                match clone_self.handle_client(stream) {
                    Ok(()) => log::info!("Disconnected"),
                    Err(err) => log::warn!("Server error: {:?}", err),
                }
            });
        }
        Ok(())
    }

    fn handle_client(self, mut stream: TcpStream) -> Result<(), BoxError> {
        let addr = stream.peer_addr().expect("Peer address");
        loop {
            // read message length
            let mut len_buf = [0; 4];
            match stream.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(err.into()),
            };

            let len = u32::from_le_bytes(len_buf) as usize;

            // read message
            let mut buf = vec![0; len];
            stream.read_exact(&mut buf)?;

            // handle the request
            let res = match self.handle_request(&addr, &buf) {
                Ok(res) => Ok(res),
                Err(err) => Err(err.to_string()),
            };

            // serialize response
            let data = serde_json::to_vec(&res)?;

            // send response
            let size: u32 = data.len().try_into()?;
            let size = size.to_le_bytes();
            stream.write_all(&size)?;
            stream.write_all(&data)?;

            // Simulate connection drop
            // let _ = stream.shutdown(std::net::Shutdown::Both);
            // break;
        }
        Ok(())
    }

    fn handle_request(&self, addr: &SocketAddr, req: &[u8]) -> Result<serde_json::Value, BoxError> {
        // TODO: Remove this.
        let _ = self;
        // Deserialize request.
        let req: T = serde_json::from_slice(req)?;
        log::trace!("Received request from {}: {:?}", addr, req);
        // handle the actual request
        let res = self.handler.handle(addr, req).map(|response| response.0);
        log::trace!("Send response to {}: {:?}", addr, res);
        Ok(res?)
    }
}

/// Handles a request and returns the corresponding response.
pub trait Handler<T> {
    /// Handle the request.
    fn handle(&self, addr: &SocketAddr, req: T) -> Result<Response, BoxError>;
}
