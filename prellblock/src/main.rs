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
    data_broadcaster::Broadcaster,
    data_storage::DataStorage,
    peer::{Calculator, Receiver},
    turi::Turi,
};
use serde::Deserialize;
use std::{
    fs,
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use structopt::StructOpt;

// https://crates.io/crates/structopt

#[derive(StructOpt, Debug)]
struct Opt {
    /// The identity name to load from config.toml file.
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Config {
    rpu: Vec<RpuConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct RpuConfig {
    name: String,
    peer_id: String,
    peer_address: SocketAddr,
    turi_address: SocketAddr,
}

#[derive(Debug, Clone, Deserialize)]
struct RpuPrivateConfig {
    identity: String, // pinxit::Identity (hex -> .key)
    tls_id: String,   // native_tls::Identity (pkcs12 -> .pfx)
}

fn main() {
    pretty_env_logger::init();
    log::info!("Kitty =^.^=");

    let opt = Opt::from_args();
    log::debug!("Command line arguments: {:#?}", opt);

    let storage = DataStorage::new(&format!("./data/{}", opt.name)).unwrap();
    let storage = Arc::new(storage);

    // load and parse config
    let config_data = fs::read_to_string("./config/config.toml").unwrap();
    let config: Config = toml::from_str(&config_data).unwrap();
    let public_config = config
        .rpu
        .iter()
        .find(|rpu_config| rpu_config.name == opt.name)
        .unwrap()
        .clone();
    let private_config_data =
        fs::read_to_string(format!("./config/{0}/{0}.toml", opt.name)).unwrap();
    let private_config: RpuPrivateConfig = toml::from_str(&private_config_data).unwrap();
    // join handles of all threads
    let mut thread_join_handles = Vec::new();

    let peer_addresses: Vec<SocketAddr> = config
        .rpu
        .iter()
        .map(|rpu_config| rpu_config.peer_address)
        .collect();

    let broadcaster = Broadcaster::new(peer_addresses);
    let broadcaster = Arc::new(broadcaster);

    // execute the turi in a new thread
    {
        let public_config = public_config.clone();
        let private_config = private_config.clone();

        thread_join_handles.push((
            format!("Turi ({})", public_config.turi_address),
            std::thread::spawn(move || {
                let listener = TcpListener::bind(public_config.turi_address).unwrap();
                let turi = Turi::new(private_config.tls_id, broadcaster);
                turi.serve(&listener).unwrap();
            }),
        ));
    }

    let calculator = Calculator::new();
    let calculator = Arc::new(calculator.into());

    // execute the receiver in a new thread
    thread_join_handles.push((
        format!("Peer Receiver ({})", public_config.peer_address),
        std::thread::spawn(move || {
            let listener = TcpListener::bind(public_config.peer_address).unwrap();
            let receiver = Receiver::new(private_config.tls_id, calculator, storage);
            receiver.serve(&listener).unwrap();
        }),
    ));

    // // execute the test client
    // if let Some(peer_addr) = opt.peer {
    //     let mut client = Sender::new(peer_addr);
    //     match client.send_request(message::Ping) {
    //         Err(err) => log::error!("Failed to send Ping: {}.", err),
    //         Ok(res) => log::debug!("Ping response: {:?}", res),
    //     }
    //     log::info!("The sum is {:?}", client.send_request(message::Add(100, 2)));
    //     log::info!(
    //         "The second sum is {:?}",
    //         client.send_request(message::Add(10, 2))
    //     );
    // }

    // wait for all threads
    for (name, join_handle) in thread_join_handles {
        match join_handle.join() {
            Err(err) => log::error!("Error occurred waiting for {}: {:?}", name, err),
            Ok(()) => log::info!("Ended {}.", name),
        };
    }
    log::info!("Going to hunt some mice. I meant *NICE*. Bye.");
}
