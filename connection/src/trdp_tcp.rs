use std::net::SocketAddr;
use std::net::IpAddr::V4;
use std::io::ErrorKind;

use futures::io::Error;
use async_trait::async_trait;
use trdp_rs::*;

use crate::listener::{Listener, Connection, Message, Connector};
use crate::trdp_lib::*;


pub struct TrdpTcpConnection {
    app: usize,
    ip: TRDP_IP_ADDR_T,
    session: Option<Session>
}

impl TrdpTcpConnection {

    /// Create a new TrdpTcpConnection instance.
    pub const fn with_session(app: usize,session: Session) -> Self {

        Self {
            app,
            ip: 0,
            session : Some(session)
        }
    }

    pub const fn with_ip(app: usize,ip: TRDP_IP_ADDR_T) -> Self {

        Self {
            app,
            ip,
            session : None
        }
    }

    pub fn get_mut_session(&mut self) -> &mut Option<Session>
    {
        return &mut self.session;
    }

}

impl Connection for TrdpTcpConnection {


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

        match self.get_mut_session() {
            Some(s) => {
                match s.get_message() {
                    Some(msg) => { return Ok(msg); },
                    None => { return Err(Error::new(ErrorKind::UnexpectedEof,"no message")); }
                }
            },
            None => panic!("No session available")
        }
        


    }

    fn write_message(&mut self,msg: &Message)
    {
        let app = self.app;
        match self.get_mut_session() {
            Some(s) => trdp_send_reply(app,&s,msg),
            None => { 
                let session_id : TRDP_UUID_T = trdp_send_request(app,self.ip,msg);
                self.session = Some(trdp_wait_response(app,session_id));
            }
        }

        
    }

}


fn as_u32_be(array: [u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}


/// A TRDP listener implementation.
///
/// The `Listener` is an wrapper for  TRDP protocol.
pub struct TrdpTcpListener {
    app : usize
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
        let app : usize = trdp_listener(own_ip,port);

        Self {
          app
        }
    }
}

#[async_trait]
impl Listener for TrdpTcpListener {


    fn port(&self) -> u16
    {
        return 0;
    }

    async fn accept(&mut self) -> Result<Box<dyn Connection>,Error>
    {
      
        let session : Session = trdp_accept(self.app).await;

        return Ok(Box::new(TrdpTcpConnection::with_session(self.app,session)));
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

        let dest_ip: TRDP_IP_ADDR_T;
        match self.address.ip() {
            V4(ip) => dest_ip = as_u32_be(ip.octets()),
            _ => panic!("ipv6 not supportet")
        }
        let port : u16 = self.address.port();
       
        let app : usize = trdp_connect(0,port);

        return Ok(Box::new(TrdpTcpConnection::with_ip(app,dest_ip)));
    }
}

