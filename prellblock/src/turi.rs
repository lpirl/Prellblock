//! A server for communicating between RPUs.

use crate::{batcher::Batcher, permission_checker::PermissionChecker};
use balise::{
    handle_fn,
    server::{Handler, Server},
    Request,
};
use prellblock_client_api::{message, ClientMessage, Pong, Transaction};
use std::{env, net::TcpListener, sync::Arc};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A receiver (server) instance.
///
/// # Example
///
/// ```no_run
/// use prellblock::turi::Turi;
/// use prellblock::permission_checker::PermissionChecker;
/// use prellblock::world_state::WorldState;
/// use prellblock::batcher::Batcher;
/// use prellblock::data_broadcaster::Broadcaster;
/// use std::{net::TcpListener, sync::Arc};
///
/// let bind_addr = "127.0.0.1:0"; // replace 0 with a real port
///
/// let listener = TcpListener::bind(bind_addr).unwrap();
/// let peer_addresses = vec!["127.0.0.1:2480".parse().unwrap()]; // The ip addresses + ports of all other peers.
///
/// let broadcaster = Broadcaster::new(peer_addresses);
/// let broadcaster = Arc::new(broadcaster);
///
/// let batcher = Batcher::new(broadcaster);
/// let batcher = Arc::new(batcher.into());
///
/// let world_state = WorldState::default();
/// let permission_checker = PermissionChecker::new(world_state);
/// let permission_checker = Arc::new(permission_checker);
///
/// let turi = Turi::new("path_to_pfx.pfx".to_string(), batcher, permission_checker);
/// std::thread::spawn(move || {
///     turi.serve(&listener).unwrap();
/// });
/// ```
#[derive(Clone)]
pub struct Turi {
    tls_identity: String,
    batcher: Arc<Batcher>,
    permission_checker: Arc<PermissionChecker>,
}

impl Turi {
    /// Create a new receiver instance.
    ///
    /// The `identity` is a path to a `.pfx` file.
    #[must_use]
    pub const fn new(
        tls_identity: String,
        batcher: Arc<Batcher>,
        permission_checker: Arc<PermissionChecker>,
    ) -> Self {
        Self {
            tls_identity,
            batcher,
            permission_checker,
        }
    }

    /// The main server loop.
    pub fn serve(self, listener: &TcpListener) -> Result<(), BoxError> {
        let tls_identity = self.tls_identity.clone();
        let password = env::var(crate::TLS_PASSWORD_ENV)
            .unwrap_or_else(|_| crate::TLS_DEFAULT_PASSWORD.to_string());
        let server = Server::new(self, tls_identity, &password)?;
        drop(password);
        server.serve(listener)
    }

    fn handle_execute(&self, params: message::Execute) -> Result<(), BoxError> {
        let message::Execute(peer_id, transaction) = params;
        // Check validity of transaction signature.
        let transaction = transaction.verify(&peer_id)?;

        // Verify permissions
        self.permission_checker.verify(&peer_id, &transaction)?;

        match &transaction as &Transaction {
            Transaction::KeyValue { key, value } => {
                log::info!("Client {} set {} to {}.", peer_id, key, value);
            }
        }

        let message = crate::peer::message::Execute(peer_id, transaction.into());
        self.batcher.clone().add_to_batch(message);

        Ok(())
    }
}

impl Handler<ClientMessage> for Turi {
    handle_fn!(self, ClientMessage, {
        Ping(_) => Ok(Pong),
        Execute(params) => self.handle_execute(params),
    });
}
