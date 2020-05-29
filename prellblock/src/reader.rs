//! A server for communicating between RPUs.

use crate::{block_storage::BlockStorage, world_state::WorldStateService, BoxError};
use prellblock_client_api::{message, ClientMessage};

type Response<R> = Result<<R as balise::Request<ClientMessage>>::Response, BoxError>;

/// The `Reader` component responds to read queries.
#[derive(Clone)]
pub struct Reader {
    block_storage: BlockStorage,
    world_state: WorldStateService,
}

impl Reader {
    /// Create a new reader instance.
    #[must_use]
    pub const fn new(block_storage: BlockStorage, world_state: WorldStateService) -> Self {
        Self {
            block_storage,
            world_state,
        }
    }

    pub(crate) async fn handle_get_value(
        &self,
        params: message::GetValue,
    ) -> Response<message::GetValue> {
        let message::GetValue(peer_ids, filter, query) = params;

        peer_ids
            .into_iter()
            .map(|peer_id| {
                let transactions =
                    self.block_storage
                        .read_transactions(&peer_id, filter.as_deref(), &query)?;
                Ok((peer_id, transactions))
            })
            .collect()
    }

    pub(crate) async fn handle_get_account(
        &self,
        params: message::GetAccount,
    ) -> Response<message::GetAccount> {
        let message::GetAccount(peer_ids) = params;

        let world_state = self.world_state.get();
        let acccounts = peer_ids
            .iter()
            .filter_map(|peer_id| world_state.accounts.get(peer_id).cloned())
            .collect();

        Ok(acccounts)
    }

    pub(crate) async fn handle_get_block(
        &self,
        params: message::GetBlock,
    ) -> Response<message::GetBlock> {
        let message::GetBlock(filter) = params;

        let blocks: Result<_, _> = self.block_storage.read(filter).collect();

        Ok(blocks?)
    }

    pub(crate) async fn handle_get_current_block_number(
        &self,
        params: message::GetCurrentBlockNumber,
    ) -> Response<message::GetCurrentBlockNumber> {
        let message::GetCurrentBlockNumber() = params;

        let world_state = self.world_state.get();
        let block_number = world_state.block_number;

        Ok(block_number)
    }
}