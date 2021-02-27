#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]
#![allow(clippy::future_not_send)]



pub mod listener;

pub use listener::Listener;
pub use listener::RawTcpListener;


