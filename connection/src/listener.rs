
use std::net::SocketAddr;
use futures::executor::block_on;
use futures::io::Error;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use async_trait::async_trait;


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
pub trait Connection: Send {

    fn local_addr(&self) -> SocketAddr;
    
    fn peer_addr(&self) -> SocketAddr;

    fn read_message(&mut self) -> Result<Message,Error>;

    fn write_message(&mut self,msg: &Message);

}

/// Interface for raw or trdp connections
#[async_trait]
pub trait Listener: Send + 'static {

    fn port(&self) -> u16;

    /// accept new connection
    async fn accept(&mut self) -> Result<Box<dyn Connection>,Error>;

}


/// Interface for raw or trdp connections
#[async_trait]
pub trait Connector: Send + 'static {

    /// create new connection
    async fn connect(&mut self) -> Result<Box<dyn Connection>,Error>;

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
  
  

    fn read_message(&mut self) -> Result<Message,Error>
    {

        let mut len_buf = [0; 4];

        let res = block_on(self.stream.read_exact(&mut len_buf));

        match res {
            Ok(_) => {},
            Err(err) => return Err(err)
        };

        log::trace!("read message: len {:?}", len_buf);

        let len = u32::from_le_bytes(len_buf) as usize;

        let mut msg_buf = vec![0; len];

        block_on(self.stream.read_exact(&mut msg_buf)).unwrap();
        log::trace!("read message: buf {:?}", msg_buf);

        return Ok(Message::new(&msg_buf));

    }

    fn write_message(&mut self,msg: &Message)
    {

        let buf  = msg.to_buffer();

        let size: u32 = buf.len() as u32;
        let mut len_buf = [0; 4];
        len_buf.copy_from_slice(&size.to_le_bytes());

        log::trace!("Write message: len {:?}", len_buf);
        block_on(self.stream.write_all(&len_buf)).unwrap();

        log::trace!("Write message: buf {:?}", buf);
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

#[async_trait]
impl Listener for RawTcpListener {


    fn port(&self) -> u16
    {
        return self.listener.local_addr().unwrap().port();
    }

    async fn accept(&mut self) -> Result<Box<dyn Connection>,Error>
    {
        let (stream ,_) = self.listener.accept().await?;

        return Ok(Box::new(RawTcpConnection::new(stream)));
    }

}


pub struct RawTcpConnector {
    address: SocketAddr
}

impl RawTcpConnector {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub const fn new(address: SocketAddr) -> Self {
        Self {
            address
        }
    }
}

#[async_trait]
impl Connector for RawTcpConnector {

    async fn connect(&mut self) -> Result<Box<dyn Connection>,Error>
    {
        let stream = TcpStream::connect(self.address).await?;
        return Ok(Box::new(RawTcpConnection::new(stream)));
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