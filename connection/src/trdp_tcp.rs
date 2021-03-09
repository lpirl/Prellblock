use std::net::SocketAddr;
use std::net::Ipv4Addr;
use std::net::IpAddr::V4;
use std::io::ErrorKind;

use futures::io::Error;
use async_trait::async_trait;
use trdp_rs::*;

use crate::listener::{Listener, Connection, Message, Connector};
use crate::trdp_lib::*;


pub struct TrdpTcpClientConnection {
    
    app: u16,
    ip: Ipv4Addr,
    session_id: TRDP_UUID_T
}

impl TrdpTcpClientConnection {

    /// Create a new TrdpTcpClientConnection instance.
    pub const fn new(app: u16,ip: Ipv4Addr) -> Self {

        Self {
            app,
            ip,
            session_id : [0;16]
        }

    }

}

fn as_u32_be(array: [u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}


impl Connection for TrdpTcpClientConnection {


    fn local_addr(&self) -> SocketAddr
    {
        return "127.0.0.1:12345".parse().unwrap();
    }

    fn peer_addr(&self) -> SocketAddr
    {
        return "127.0.0.1:12345".parse().unwrap();
    }
  
    

    fn read_message(&mut self) -> Result<Message,Error>
    {
        let mut session = trdp_wait_response(self.app,self.session_id);
        match session.get_message() {
            Some(msg) => { return Ok(msg); },
            None => { return Err(Error::new(ErrorKind::UnexpectedEof,"no message")); }
        }
    }

    fn write_message(&mut self,msg: &Message)
    {
        //request 
        self.session_id = trdp_send_request(self.app,as_u32_be(self.ip.octets()),msg);
    }

}

impl Drop for TrdpTcpClientConnection {
    fn drop(&mut self) {
        trdp_disconnect(self.app);
    }
}

pub struct TrdpTcpListenerConnection {
    
    app: u16,
    session: Session
}

impl TrdpTcpListenerConnection {

    /// Create a new TrdpTcpConnection instance.
    pub const fn new(app: u16,session: Session) -> Self {
        Self {
            app,
            session 
        }
    }

}

impl Connection for TrdpTcpListenerConnection  {


    fn local_addr(&self) -> SocketAddr
    {
        return "127.0.0.1:12345".parse().unwrap();
    }

    fn peer_addr(&self) -> SocketAddr
    {
        return "127.0.0.1:12345".parse().unwrap();
    }
  
    

    fn read_message(&mut self) -> Result<Message,Error>
    {
        match self.session.get_message() {
            Some(msg) => { return Ok(msg); },
            None => { return Err(Error::new(ErrorKind::UnexpectedEof,"no message")); }
        }
    }

    fn write_message(&mut self,msg: &Message)
    {
       trdp_send_reply(self.app,&self.session,msg);
      
        
    }

}




/// A TRDP listener implementation.
///
/// The `Listener` is an wrapper for  TRDP protocol.
pub struct TrdpTcpListener {
    app : u16,
    port : u16
}


impl TrdpTcpListener {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub fn new(peer_address: SocketAddr) -> Self {
        
        let own_ip: TRDP_IP_ADDR_T;
        match peer_address.ip() {
            V4(ip) => own_ip = as_u32_be(ip.octets()),
            _ => panic!("ipv6 not supportet")
        }
        let port : u16 = peer_address.port();
        let app : u16 = trdp_listener(own_ip,port);

        Self {
          app,
          port
        }
    }
}

#[async_trait]
impl Listener for TrdpTcpListener {


    fn port(&self) -> u16
    {
        return self.port;
    }

    async fn accept(&mut self) -> Result<Box<dyn Connection>,Error>
    {
        let session : Session = trdp_accept(self.app).await?;

        return Ok(Box::new(TrdpTcpListenerConnection::new(self.app,session)));
    }

}


pub struct TrdpTcpConnector {
    address: SocketAddr
}

impl TrdpTcpConnector {

    /// Create a new RawTcpListener instance.
    #[must_use]
    pub const fn new(address: SocketAddr) -> Self {
        Self {
            address
        }
    }
}

#[async_trait]
impl Connector for TrdpTcpConnector {

    async fn connect(&mut self) -> Result<Box<dyn Connection>,Error>
    {
        let dest_ip: Ipv4Addr;
        match self.address.ip() {
            V4(ip) => dest_ip = ip,
            //V6(ip6) => dest_ip = ip6.to_ipv4(), localhost => Some(0.0.0.1) ?Makes sense?
            _ => panic!("ipv6 not supportet")
        }


        let port : u16 = self.address.port();
       
        let app : u16 = trdp_connect(0,port);

        log::info!("Connect to {}:{}",self.address.ip(),port);

        return Ok(Box::new(TrdpTcpClientConnection::new(app,dest_ip)));
    }
}

