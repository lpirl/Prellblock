use connection::listener::{ Message, Connection , Connector, RawTcpConnector };
use futures::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::env;

async fn process(mut connection: Box<dyn Connection>) {


    println!("Connected: {}", connection.peer_addr());
    let msg : Message = Message::new(&[1,2,3,4,5]);
    connection.write_message(&msg);


    let mut result : Message = connection.read_message().unwrap();
    println!("Receive: {}", result.to_hex());

}

#[tokio::main]
async fn main() -> Result<(),Error> {

    let args: Vec<String> = env::args().collect();

    let mut port : u16 = 8080;
    match args.len() {
        1 => {},
        2 => {
            match args[1].parse() {
                Ok(n) => port = n,
                _ => panic!{"Give port argument"},

            }
        },
        _ => panic!{"Give port argument"},
       
    }

    println!{"Connect to port {}",port}


    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port );
    let mut connector = RawTcpConnector::new(addr);

    let connection = connector.connect().await?;

    process(connection).await;
    
    Ok(())
}