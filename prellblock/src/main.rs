#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

//! Bahndaten verlässlich und schnell in die Blockchain gepuffert - **Persistente Redundante Einheit für Langzeit-Logging über Blockchain**
//!
//! ## Overview
//!
//! `PrellBlock` is a lightweight logging blockchain, written in `Rust`, which is designed for datastorage purposes in a railway environment.
//! By using an execute-order-validate procedure it is assured, that data will be saved, even in case of a total failure of all but one redundant processing unit.
//! While working in full capactiy, data is stored and validated under byzantine fault tolerance. This project is carried out in cooperation with **Deutsche Bahn AG**.

use prellblock::{
    peer::{message, Calculator, Receiver, Sender},
    turi::Turi,
};
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use structopt::StructOpt;

// https://crates.io/crates/structopt

#[derive(StructOpt, Debug)]
struct Opt {
    /// The address on which to open the RPU communication server.
    #[structopt(
        short,
        long,
        help = "The Address and port on which to bind the RPU Receiver."
    )]
    bind: Option<SocketAddr>,

    #[structopt(
        short,
        long,
        help = "The peer to communicate with through the RPU Sender."
    )]
    peer: Option<SocketAddr>,

    #[structopt(long, help = "The address and port on which to bind the Turi.")]
    turi: Option<SocketAddr>,

    #[structopt(
        short = "c",
        long = "cert",
        help = "Path to a .pfx certificate identity signed by the CA."
    )]
    tls_identity: Option<String>,
}

fn main() {
    pretty_env_logger::init();
    log::info!("Kitty =^.^=");

    let opt = Opt::from_args();
    log::debug!("Command line arguments: {:#?}", opt);

    // execute the turi in a new thread

    let turi_handle = if let Some(turi_addr) = opt.turi {
        if let Some(tls_identity) = opt.tls_identity.clone() {
            Some(std::thread::spawn(move || {
                let listener = TcpListener::bind(turi_addr).unwrap();
                let turi = Turi::new(tls_identity);
                turi.serve(&listener).unwrap();
            }))
        } else {
            log::error!("No TLS identity given for Turi.");
            None
        }
    } else {
        None
    };

    let calculator = Calculator::new();
    let calculator = Arc::new(calculator.into());

    // execute the rpu server in a new thread
    let server_handle = if let Some(bind_addr) = opt.bind {
        if let Some(tls_identity) = opt.tls_identity.clone() {
            Some(std::thread::spawn(move || {
                let listener = TcpListener::bind(bind_addr).unwrap();
                let server = Receiver::new(calculator, tls_identity);
                server.serve(&listener).unwrap();
            }))
        } else {
            log::error!("No TLS identity given for Receiver.");
            None
        }
    } else {
        None
    };

    // execute the test client
    if let Some(peer_addr) = opt.peer {
        let mut client = Sender::new(peer_addr);
        match client.send_request(message::Ping) {
            Err(err) => log::error!("Failed to send Ping: {}.", err),
            Ok(res) => log::debug!("Ping response: {:?}", res),
        }
        log::info!("The sum is {:?}", client.send_request(message::Add(100, 2)));
        log::info!(
            "The second sum is {:?}",
            client.send_request(message::Add(10, 2))
        );
    }

    // wait for the turi thread
    if let Some(turi_handle) = turi_handle {
        match turi_handle.join() {
            Err(err) => log::error!("Turi error occured: {:?}", err),
            Ok(()) => log::info!("No error occured. Going to hunt some mice. I meant *NICE*. Bye."),
        };
    }

    // wait for the server thread
    if let Some(server_handle) = server_handle {
        match server_handle.join() {
            Err(err) => log::error!("Server error occured: {:?}", err),
            Ok(()) => log::info!("No error occured. Going to hunt some mice. I meant *NICE*. Bye."),
        };
    }
}
