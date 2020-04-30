#![warn(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

//! Library Crate used for Communication between external Clients and internal RPUs.

use balise::define_api;
use pinxit::{Signable, Signed};
use serde::{Deserialize, Serialize};

/// Play ping pong. See [`Ping`](message/struct.Ping.html).
#[derive(Debug, Serialize, Deserialize)]
pub struct Pong;

define_api! {
    /// The message API module for communication between RPUs.
    mod message;
    /// One of the requests.
    pub enum ClientMessage {
        /// Ping Message. See [`Pong`](../struct.Pong.html).
        Ping => Pong,
        /// Simple transaction Message. Will write a key:value pair.
        Execute(Signed<Transaction>) => (),
    }
}

/// A blockchain transaction for prellblock.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Transaction {
    /// Set a `key` to a `value`.
    KeyValue {
        /// The key.
        key: String,
        /// The value.
        value: Vec<u8>,
    },
}

impl Signable for Transaction {
    type SignableData = Vec<u8>;
    type Error = postcard::Error;
    fn signable_data(&self) -> Result<Self::SignableData, Self::Error> {
        postcard::to_stdvec(self)
    }
}
