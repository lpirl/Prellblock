#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::future_not_send,
    clippy::missing_errors_doc,
    clippy::similar_names
)]

//! An example client used to simulate clients.

mod cli;

use cli::prelude::*;
use prellblock_client::{account::Permissions, Client, Query};
use noise::{NoiseFn, Perlin};
use pinxit::Identity;
use rand::{
    rngs::{OsRng, StdRng},
    seq::SliceRandom,
    thread_rng, RngCore, SeedableRng,
};
use serde::Deserialize;
use std::{
    fmt, fs,
    net::SocketAddr,
    str,
    str::FromStr,
    time::{Duration, Instant},
};
use structopt::StructOpt;
use tokio::time::{delay_until, timeout};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Little Kitty =^.^=");

    let opt = Opt::from_args();
    log::debug!("Command line arguments: {:#?}", opt);

    match opt.cmd {
        Cmd::Set(cmd) => main_set(cmd).await,
        Cmd::Benchmark(cmd) => main_benchmark(cmd).await,
        Cmd::UpdateAccount(cmd) => main_update_account(cmd).await,
        Cmd::GetValue(cmd) => main_get_value(cmd).await,
        Cmd::GetAccount(cmd) => main_get_account(cmd).await,
        Cmd::GetBlock(cmd) => main_get_block(cmd).await,
        Cmd::CurrentBlockNumber => main_current_block_number().await,
    }
}

fn create_client(turi_address: SocketAddr, identity: &str) -> Client {
    let identity = identity.parse().unwrap();
    Client::new(turi_address, identity)
}

fn writer_client(turi_address: SocketAddr) -> Client {
    create_client(
        turi_address,
        "406ed6170c8672e18707fb7512acf3c9dbfc6e5ad267d9a57b9c486a94d99dcc",
    )
#[derive(StructOpt, Debug)]
enum Command {
    /// Transaction to set a key to a value.
    Set {
        /// The key of this transaction.
        key: String,
        /// The value of the corresponding key.
        value: String,
    },
    /// Benchmark the blockchain.
    #[structopt(name = "bench")]
    Benchmark {
        /// The name of the RPU to benchmark.
        rpu_name: String,
        /// The key to use for saving benchmark generated data.
        key: String,
        /// The number of transactions to send
        transactions: u32,
        /// The number of bytes each transaction's payload should have.
        #[structopt(short, long, default_value = "8")]
        size: usize,
        /// The number of workers (clients) to use simultaneously.
        #[structopt(short, long, default_value = "1")]
        workers: usize,
    },
    /// Generate and write values to the blockchain.
    #[structopt(name = "gen")]
    Generate {
        /// The duration to generate data for. (in ms)
        #[structopt(short, long, default_value = "60000")]
        duration: u64,
        /// The interval to generate and write a new value. (in ms)
        #[structopt(short, long, default_value = "100")]
        interval: u64,
        /// The number of bytes each transaction's payload should have.       
        gen_type: GeneratorType,
    },
    /// Listen to timeseries and push updates via POST.
    Listen {
        /// The interval to poll values from timeseries. (in ms)
        #[structopt(short, long, default_value = "100")]
        polling_interval: u64,
    },
    /// Update the permissions for a given account.
    #[structopt(name = "update")]
    UpdateAccount {
        /// The id of the account to update.
        id: String,
        /// The filepath to a yaml-file cotaining the accounts permissions.
        permission_file: String,
    },
}

#[derive(Debug, Clone)]
enum GeneratorType {
    Temperature,
}

#[derive(Debug, Clone)]
struct GeneratorParseError;

impl fmt::Display for GeneratorParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid GeneratorType")
    }
}

impl FromStr for GeneratorType {
    type Err = GeneratorParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Temperature" => Ok(Self::Temperature),
            _ => Err(GeneratorParseError),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Config {
    rpu: Vec<RpuConfig>,
}

fn reader_client() -> Client {
    let mut rng = thread_rng();
    let turi_address = Config::load().rpu.choose(&mut rng).unwrap().turi_address;

    // matching peerid is: cb932f482dc138a76c6f679862aa3692e08c140284967f687c1eaf75fd97f1bc
    create_client(
        turi_address,
        "03d738c972f37a6fd9b33278ac0c50236e45637bcd5aeee82d8323655257d256",
    )
}

async fn main_set(cmd: cmd::Set) {
    let cmd::Set { key, value } = cmd;

    let mut rng = thread_rng();
    let turi_address = Config::load().rpu.choose(&mut rng).unwrap().turi_address;

    // execute the test client
    match writer_client(turi_address).send_key_value(key, value).await {
        Err(err) => log::error!("Failed to send transaction: {}", err),
        Ok(()) => log::debug!("Transaction ok!"),
    }
}

async fn main_benchmark(cmd: cmd::Benchmark) {
    let cmd::Benchmark {
        rpu_name,
        key,
        transactions,
        size,
        workers,
    } = cmd;

    let turi_address = Config::load()
        .rpu
        .iter()
        .find(|rpu| rpu.name == rpu_name)
        .unwrap()
        .turi_address;

    let mut worker_handles = Vec::new();
    for _ in 0..workers {
        let key = key.clone();
        worker_handles.push(tokio::spawn(async move {
            let mut rng = StdRng::from_rng(OsRng {}).unwrap();
            let mut client = writer_client(turi_address);

            let start = Instant::now();
            let half_size = (size + 1) / 2;
            let mut bytes = vec![0; half_size];
            let mut value = vec![0; half_size * 2];
            for _ in 0..transactions {
                let key = key.clone();
                // generate random data (hex)
                rng.fill_bytes(&mut bytes);
                hex::encode_to_slice(&bytes, &mut value).unwrap();
                let value = str::from_utf8(&value[..size]).unwrap();
                match client.send_key_value(key, value).await {
                    Err(err) => log::error!("Failed to send transaction: {}", err),
                    Ok(()) => log::debug!("Transaction ok!"),
                }
            }
            start.elapsed()
        }));
    }

    for (n, worker) in worker_handles.into_iter().enumerate() {
        if let Ok(time_diff) = worker.await {
            let avg_time_per_tx = time_diff / transactions;
            let avg_tps = 1.0 / avg_time_per_tx.as_secs_f64();
            log::info!(
                "--------------------------------------------------------------------------------"
            );
            log::info!("Finished benchmark with worker {}.", n);
            log::info!("Number of transactions: {}", transactions);
            log::info!("Transaction size:       {} bytes", size);
            log::info!(
                "Sum of sent payload:    {} bytes",
                size * transactions as usize
            );
            log::info!("Duration:               {:?}", time_diff);
            log::info!("Transaction time:       {:?}", avg_time_per_tx);
            log::info!("TPS (averaged):         {}", avg_tps);
        } else {
            log::error!("Failed to benchmark with worker {}", n);
        }
    }
}

async fn main_update_account(cmd: cmd::UpdateAccount) {
    let cmd::UpdateAccount {
        id,
        permission_file,
    } = cmd;

    // TestCLI: 256cdb0197402705f96d39eab7dd3d47a39cb75673a58852d83f666973d80e01
    let id = id.parse().expect("Invalid account id given");

    // Read `Permissions` from the given file.
    let permission_file_content =
        fs::read_to_string(permission_file).expect("Could not read permission file");
    let permissions: Permissions =
        serde_yaml::from_str(&permission_file_content).expect("Invalid permission file content");

    match reader_client().update_account(id, permissions).await {
        Err(err) => log::error!("Failed to send transaction: {}", err),
        Ok(()) => log::debug!("Transaction ok!"),
    }
}

async fn main_get_value(cmd: cmd::GetValue) {
    let cmd::GetValue {
        peer_id,
        filter,
        span,
        end,
        skip,
    } = cmd;

    let query = Query::Range {
        span: span.0,
        end: end.0,
        skip: skip.map(|skip| skip.0),
    };

    match reader_client()
        .query_values(vec![peer_id], filter.0, query)
        .await
    {
        Ok(values) => {
            if values.is_empty() {
                log::warn!("No values retrieved.");
            }

            for (peer_id, values_of_peer) in values {
                if values_of_peer.is_empty() {
                    log::warn!("No values retrieved for peer {}.", peer_id);
                } else {
                    log::info!("The retrieved values of peer {} are:", peer_id);
                }
                for (key, values_by_key) in values_of_peer {
                    if values_by_key.is_empty() {
                        log::warn!("No values retrieved for key {:?}.", key);
                    } else {
                        log::info!("  Key {:?}:", key);
                    }
                    for (timestamp, value) in values_by_key {
                        log::info!(
                            "    {}: {:?}",
                            humantime::format_rfc3339_millis(timestamp),
                            value
                        );
                    }
                }
            }
        }
<<<<<<< HEAD
        Err(err) => log::error!("Failed to retrieve values: {}", err),
    }
}

async fn main_get_account(cmd: cmd::GetAccount) {
    let cmd::GetAccount { peer_ids } = cmd;

    match reader_client().query_account(peer_ids).await {
        Ok(accounts) => {
            if accounts.is_empty() {
                log::warn!("No accounts retrieved.");
            } else {
                log::info!("The retrieved accounts are:");
            }
            for account in accounts {
                log::info!("{:#?}", account);
            }
        }
        Err(err) => log::error!("Failed to retrieve accounts: {}", err),
    }
}

async fn main_get_block(cmd: cmd::GetBlock) {
    let cmd::GetBlock { filter } = cmd;

    match reader_client().query_block(filter.0).await {
        Ok(block_vec) => {
            if block_vec.is_empty() {
                log::warn!("No blocks retrieved for the given range.");
            } else {
                log::info!("The retrieved blocks are:");
            }
            for block in block_vec {
                log::info!("{:#?}", block);
            }
        }
        Err(err) => log::error!("Failed to retrieve blocks: {}", err),
    }
}

async fn main_current_block_number() {
    match reader_client().current_block_number().await {
        Err(err) => log::error!("Failed to retrieve current block number: {}", err),
        Ok(block_number) => log::info!(
            "The current block number is: {:?}. The last committed block number is: {:?}.",
            block_number,
            block_number - 1
        ),
    }
}
//         Command::Generate {
//             duration,
//             interval,
//             gen_type,
//         } => {
//             let mut rng = thread_rng();
//             let turi_address = config.rpu.choose(&mut rng).unwrap().turi_address;

//             // matching peerid is: cb932f482dc138a76c6f679862aa3692e08c140284967f687c1eaf75fd97f1bc
//             let identity: Identity =
//                 "03d738c972f37a6fd9b33278ac0c50236e45637bcd5aeee82d8323655257d256"
//                     .parse()
//                     .unwrap();

//             let mut client = Client::new(turi_address, identity);
//             let res = timeout(
//                 Duration::from_millis(duration),
//                 generate_data(&mut client, gen_type.clone(), interval),
//             )
//             .await;

//             if res.is_err() {
//                 println!("Done generating.");
//             }
//         }
//         Command::UpdateAccount {
//             id,
//             permission_file,
//         } => {
//             let mut rng = thread_rng();
//             let turi_address = config.rpu.choose(&mut rng).unwrap().turi_address;

//             // matching peerid is: cb932f482dc138a76c6f679862aa3692e08c140284967f687c1eaf75fd97f1bc
//             let identity: Identity =
//                 "03d738c972f37a6fd9b33278ac0c50236e45637bcd5aeee82d8323655257d256"
//                     .parse()
//                     .unwrap();

//             let mut client = Client::new(turi_address, identity);

//             // TestCLI: 256cdb0197402705f96d39eab7dd3d47a39cb75673a58852d83f666973d80e01
//             let id = id.parse().expect("Invalid account id given");

//             // Read `Permissions` from the given file.
//             let permission_file_content =
//                 fs::read_to_string(permission_file).expect("Could not read permission file");
//             let permissions: Permissions = serde_yaml::from_str(&permission_file_content)
//                 .expect("Invalid permission file content");

//             match client.update_account(id, permissions).await {
//                 Err(err) => log::error!("Failed to send transaction: {}", err),
//                 Ok(()) => log::debug!("Transaction ok!"),
//             }
//         }
//         Command::Listen { polling_interval } => {
//             // load timeseries from config
//         }
//     }
// }

// async fn generate_data(client: &mut Client, gen_type: GeneratorType, interval: u64) {
//     let start = Instant::now();
//     let perlin = Perlin::new();

//     loop {
//         let deadline = tokio::time::Instant::now() + Duration::from_millis(interval);
//         // println!("deadline: {:?}", deadline);
//         match gen_type {
//             GeneratorType::Temperature => {
//                 let time = Instant::now() - start;
//                 let time = 1.01 * time.as_millis() as f64;
//                 let mut value = 20 as f64 + 10.0 * perlin.get([time, time, time]);
//                 value = (value * 100.0).round() / 100.0;
//                 // println!("time: {}", time);
//                 // println!("temp: {}", value);
//                 match client
//                     .send_key_value("temperature".to_string(), value)
//                     .await
//                 {
//                     Err(err) => log::error!("Failed to send transaction: {}", err),
//                     Ok(()) => log::debug!("Transaction ok!"),
//                 }
//             }
//         }
//         let _ = delay_until(deadline).await;
// >>>>>>> Add data generation feature to CLI
//     }
// }
