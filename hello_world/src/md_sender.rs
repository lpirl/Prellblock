use connection::listener::{ Message, Connection , Connector };
use connection::trdp_tcp::{ TrdpTcpConnector };

use futures::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::env;

use tokio::time::{sleep, Duration};


async fn process(mut connection: Box<dyn Connection>) {


    log::info!("Connected to {}", connection.peer_addr());

    let mut msg : Message = Message::new(&[1,2,3,4,5]);
    connection.write_message(&msg);
    log::info!("Wrote request {}", msg.to_hex());

    let mut result : Message = connection.read_message().unwrap();
    log::info!("Receive response {}", result.to_hex());
    
}

#[tokio::main]
async fn main() -> Result<(),Error> {

    pretty_env_logger::init();
    log::info!("Sender ♥");

    let args: Vec<String> = env::args().collect();

    let mut port : u16 = 17225;
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



    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port );
    
    let mut connector = TrdpTcpConnector::new(addr);


    for a in 0..10 {
     
        let connection = connector.connect().await?;
        process(connection).await;
        //Todo close 
        sleep(Duration::from_millis(1000)).await;

    }
    Ok(())
}