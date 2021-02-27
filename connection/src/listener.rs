
use std::net::SocketAddr;
use futures::executor::block_on;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use hex::encode;

pub struct Message {
    data : Vec<u8>
} 

impl Message {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub fn new(buf: &[u8]) -> Self {

        Self {
            data: buf.iter().cloned().collect()
        }
    }

    pub fn to_buffer(&self) -> &Vec<u8> {
        return &self.data;
    }

    pub fn to_hex(&mut self) -> String
    {
        //clone
        let d : Vec<u8> = self.data.iter().cloned().collect();
        return hex::encode(d);
    }
}
/// Communication interface to peer 
pub trait Connection {

    fn local_addr(&self) -> SocketAddr;
    
    fn peer_addr(&self) -> SocketAddr;

    fn read_message(&mut self) -> Message;

    fn write_message(&mut self,msg: &Message);

}

/// Interface for raw or trdp connections
pub trait Listener: Send + 'static {

    fn port(&self) -> u16;

    /// accept new connection
    fn accept(&mut self) -> Box<dyn Connection>;

}

pub struct RawTcpConnection {
    stream: TcpStream
}

impl RawTcpConnection {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub const fn new(stream: TcpStream) -> Self {

        Self {
            stream 
        }
    }
}

impl Connection for RawTcpConnection {


    fn local_addr(&self) -> SocketAddr
    {
        return self.stream.local_addr().expect("Local address");
    }

    fn peer_addr(&self) -> SocketAddr
    {
        return self.stream.peer_addr().expect("Peer address");
    }
  
  

    fn read_message(&mut self) -> Message
    {

        let mut len_buf = [0; 4];

        block_on(self.stream.read_exact(&mut len_buf)).unwrap();

        let len = u32::from_le_bytes(len_buf) as usize;

        let mut msg_buf = vec![0; len];

        block_on(self.stream.read_exact(&mut msg_buf)).unwrap();

        return Message::new(&msg_buf);

    }

    fn write_message(&mut self,msg: &Message)
    {

        let buf  = msg.to_buffer();

        let size: u32 = buf.len() as u32;
        let mut len_buf = [0; 4];
        len_buf.copy_from_slice(&size.to_le_bytes());

        block_on(self.stream.write_all(&len_buf)).unwrap();

        block_on(self.stream.write_all(&buf)).unwrap();


       

    }

}




/// A raw tcp listener implementation.
///
/// The `Listener` is an wrapper for raw Tcp.
pub struct RawTcpListener {
    listener: TcpListener
}

impl RawTcpListener {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub fn new(peer_address: SocketAddr) -> Self {
        
        let listener =  block_on(TcpListener::bind(peer_address)).unwrap();

        Self {
            listener
        }
    }
}

impl Listener for RawTcpListener {


    fn port(&self) -> u16
    {
        return self.listener.local_addr().unwrap().port();
    }

    fn accept(&mut self) -> Box<dyn Connection>
    {
        let stream_future = self.listener.accept();
        let (stream,_) = block_on(stream_future).unwrap();

        //TODO add tls layer
        
        return Box::new(RawTcpConnection::new(stream));
    }

}

/*
/// A TRDP listener implementation.
///
/// The `Listener` is an wrapper for  TRDP protocol.
pub struct TrdpTcpListener {
    //peer_address: SocketAddr
}

impl Listener for TrdpTcpListener {
   
    fn init(&mut self)
    {
        println!("init");
    }

    fn port(&self) -> u16
    {
        return 0;
    }

    fn accept(&mut self) -> Box<dyn Connection>
    {
        unimplemented!("accept not implemented");
    }
}

*/