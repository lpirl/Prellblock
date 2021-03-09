//! A client for communicating between RPUs.

mod connection_pool;

use crate::{Address, Error, Request};
use serde::Serialize;
use std::{
    convert::TryInto,
    marker::{PhantomData, Unpin},
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, Instant},
};
use std::fmt::Write;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};


use connection::listener::{ Message, Connection , Connector, RawTcpConnector };

use connection::trdp_tcp::TrdpTcpConnector;

/// A client instance.
///
/// The client keeps up a connection pool of open connections
/// for improved efficiency.
pub struct Client<T> {
    addr: Address,
    request_data: PhantomData<T>,
}

impl<T> Client<T> {
    /// Create a new client instance.
    ///
    /// # Example
    ///
    /// ```
    /// use balise::client::Client;
    ///
    /// let addr = "127.0.0.1:2480".parse().unwrap();
    /// let client = Client::<()>::new(addr);
    /// ```
    #[must_use]
    pub const fn new(addr: Address) -> Self {
        Self {
            addr,
            request_data: PhantomData,
        }
    }

    /// Send a request to the server specified.
    pub async fn send_request<Req>(&mut self, req: Req) -> Result<Req::Response, Error>
    where
        Req: Request<T>,
        T: Serialize,
    {

        let mut adr_str = String::new();
        write!(&mut adr_str, "{}", self.addr).expect("Unable to write");
        
        let resolved_addresses: Vec<_> = adr_str
            .to_socket_addrs()
            .expect("Unable to resolve peer address")
            .collect();
        let resolved_address = resolved_addresses.first().unwrap();
        let address = *resolved_address;
        //let address = SocketAddr::new(self.addr.host.to_string(), self.addr.port);

        let mut connector = TrdpTcpConnector::new(address);
        let connection = connector.connect().await?;

        //let (mut stream, addr) = self.stream().await?;

        log::trace!("Sending request to {}: {:?}", address, req);
        let res = send_request(connection, req).await?;

        log::trace!("Received response from {}: {:?}", address, res);
        //stream.done().await;
        Ok(res?)
    }

    /// Get a working TCP stream.
    ///
    /// A stream could be closed by the receiver while being
    /// in the pool. This is catched and a new stream will be
    /// returned in this case.
    async fn stream(&self) -> Result<(connection_pool::StreamGuard<'_>, SocketAddr), Error> {
        let deadline = Instant::now() + Duration::from_secs(3);
        let delay = Duration::from_secs(1);

        let res = loop {
            if Instant::now() > deadline {
                return Err(Error::Timeout);
            }

            let stream = match connection_pool::POOL.stream(self.addr.clone()).await {
                Ok(stream) => stream,
                Err(err) => {
                    log::warn!(
                        "Couldn't connect to server at {}, retrying in {:?}: {}",
                        self.addr,
                        delay,
                        err
                    );
                    std::thread::sleep(delay);
                    continue;
                }
            };
            let addr = stream.tcp_stream().peer_addr()?;

            // // check TCP connection functional
            // stream.tcp_stream().set_nonblocking(true)?;

            // //read one byte without removing from message queue
            // let mut buf = [0; 1];
            // match stream.tcp_stream().peek(&mut buf) {
            //     Ok(n) => {
            //         if n > 0 {
            //             log::warn!("The Receiver is not working correctly!");
            //         }
            //         // no connection
            //         let local_addr = stream.tcp_stream().local_addr().unwrap();
            //         log::trace!(
            //             "TCP connection from {} to {} seems to be broken.",
            //             local_addr,
            //             addr
            //         );
            //     }
            //     Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            //         // blocking means stream is ok
            //         stream.tcp_stream().set_nonblocking(false)?;
            //         break (stream, addr);
            //     }
            //     Err(e) => return Err(e.into()),
            // }
            break (stream, addr);
        };
        Ok(res)
    }
}

async fn send_request<Req, T>(
    mut connection: Box<dyn Connection>,
    req: Req,
) -> Result<Result<Req::Response, String>, Error>
where
    Req: Request<T>,
    T: Serialize,
{


    let req: T = req.into();

    // serialize request
    let vec = vec![0; 0];
    let vec = postcard::serialize_with_flavor(&req,postcard::flavors::StdVec(vec))?;

    let mut resp_message : Message = Message::new(&vec);
            
    connection.write_message(&resp_message);

    let recv_message : Message = connection.read_message().unwrap();
           

    let buf = recv_message.to_buffer();   //vec![0; len];

    let res = match postcard::from_bytes(&buf)? {
        Ok(data) => Ok(postcard::from_bytes(data)?),
        Err(err) => Err(err),
    };
    Ok(res)
}
