//! Module used for Broadcasting Messages between all RPUs.

use crate::{
    peer::{PeerMessage, Sender},
    world_state::WorldStateService,
    BoxError,
};
use balise::Request;
use futures::future::join_all;

/// A broadcaster for peer messages.
pub struct Broadcaster {
    world_state: WorldStateService,
}

impl Broadcaster {
    /// Create a new Broadcaster
    ///
    /// `world_state` should be a `WorldState` containing all other RPUs peer addresses.
    #[must_use]
    pub const fn new(world_state: WorldStateService) -> Self {
        Self { world_state }
    }

    /// Broadcast a batch to all known peers (stored in `peer_addresses`).
    #[allow(clippy::future_not_send)]
    pub async fn broadcast<T>(&self, message: &T) -> Result<(), BoxError>
    where
        T: Request<PeerMessage>,
    {
        // Broadcast transaction to all RPUs.
        let results = join_all(
            self.world_state
                .get()
                .peers
                .iter()
                .map(|(_, peer_address)| {
                    let message = message.clone();
                    let peer_address = *peer_address;
                    tokio::spawn(async move {
                        log::trace!("Sending batch to {}.", peer_address);
                        let mut sender = Sender::new(peer_address);
                        let result = sender.send_request(message).await;
                        log::trace!("Sent batch to {}.", peer_address);
                        result
                    })
                }),
        )
        .await;

        for result in results {
            // Ignore result
            let _ = result??;
        }
        Ok(())
    }
}
