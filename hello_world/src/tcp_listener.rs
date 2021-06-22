use connection::listener::{ Message, Connection , Listener };
use connection::listener::{ RawTcpListener };

use futures::io::Error;
use futures::future;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

async fn process(mut connection: Box<dyn Connection>) {

    log::info!("Connection from {}", connection.peer_addr());

    let mut msg : Message = connection.read_message().unwrap();
    log::info!("Receive request {}", msg.to_hex());

    connection.write_message(&msg);
    log::info!("Wrote response {}", msg.to_hex());

}


async fn serve(addr : SocketAddr) -> Result<(), Error> {
    let mut listener = RawTcpListener::new(addr);
    loop {
            let mut connection = listener.accept().await?;
        
            log::info!("Connection from {}", connection.peer_addr());
            tokio::spawn(async move {
                    // Process each socket concurrently.
                    log::info!("Connection from {}", connection.peer_addr());

                    process(connection).await;
            });
    }
}

#[tokio::main]
async fn main() {

    pretty_env_logger::init();
    log::info!("Listener ♥");




    let task1 = {
        tokio::spawn(async move {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 17225);
            serve(addr).await;
        })
    };


    let task2 = {
      
        tokio::spawn(async move {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
            serve(addr).await;
        })
    };


    // wait for all tasks
    future::join(
        async move {
            log::error!("task1 ended: {:?}", task1.await);
        },
        async move {
            log::error!("task2 ended: {:?}", task2.await);
        },
    )
    .await;

    
}