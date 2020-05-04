use super::state::Phase;
use crate::{
    consensus::{BlockNumber, LeaderTerm},
    permission_checker::PermissionError,
};
use err_derive::Error;
use pinxit::PeerId;

/// An error of the `praftbft` consensus.
#[allow(clippy::pub_enum_variant_names)]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(display = "{}", 0)]
    PermissionError(#[error(from)] PermissionError),

    /// The signature could not be verified.
    #[error(display = "{}", 0)]
    InvalidSignature(#[error(from)] pinxit::Error),

    /// The `Follower`'s leader_term is not equal to the received leader_term.
    #[error(display = "Follower is not in the correct Leader term.")]
    WrongLeaderTerm,

    /// The leader proposing the block is not the one the `Follower` saved (maybe there is no leader).
    #[error(display = "There is no leader.")]
    NoLeader,

    /// The Block Hash has changed between Phases.
    #[error(display = "The Block Hash has changed.")]
    ChangedBlockHash,

    /// The Block Hash is wrong.
    #[error(display = "The Block Hash is wrong.")]
    WrongBlockHash,

    /// The leader proposing the block is not the one the `Follower` saved (maybe there is no leader).
    #[error(display = "The RPU {} is not the current leader.", 0)]
    WrongLeader(PeerId),

    /// The current block number is already higher.
    #[error(display = "Block number {} is too low.", 0)]
    BlockNumberTooSmall(BlockNumber),

    /// The current block number is already higher.
    #[error(display = "Block number {} is too big.", 0)]
    BlockNumberTooBig(BlockNumber),

    /// The current block number is already higher.
    #[error(display = "Request ViewChange to term {} failed: term too low.", 0)]
    LeaderTermTooSmall(LeaderTerm),

    /// The current block number is already higher.
    #[error(display = "Request ViewChange to term {} failed: term too high.", 0)]
    LeaderTermTooBig(LeaderTerm),

    /// The current block number is different from the expected one.
    #[error(display = "Block number {} is wrong.", 0)]
    WrongBlockNumber(BlockNumber),

    /// This peer is not allowed to take part in the consensus.
    #[error(
        display = "The RPU {} is not allowed to take part in the consensus.",
        0
    )]
    InvalidPeer(PeerId),

    #[error(display = "Not enough signatures.")]
    NotEnoughSignatures,

    #[error(
        display = "Expected to be in {:?} phase but was in {:?} phase.",
        expected,
        current
    )]
    WrongPhase {
        current: PhaseName,
        expected: PhaseName,
    },
}

#[derive(Debug)]
pub enum PhaseName {
    Waiting,
    Prepare,
    Append,
    Commited,
}

impl Phase {
    /// Convert a phase to the corresponding `PhaseName`.
    pub(super) fn to_phase_name(&self) -> PhaseName {
        match self {
            Self::Waiting => PhaseName::Waiting,
            Self::Prepare(..) => PhaseName::Prepare,
            Self::Append(..) => PhaseName::Append,
            Self::Committed(..) => PhaseName::Commited,
        }
    }
}
