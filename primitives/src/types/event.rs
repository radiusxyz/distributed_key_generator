use crate::{EncKeyCommitment, SessionId};
use std::time::Instant;

/// Event of this node
#[derive(Debug, Clone)]
pub enum Event<Signature, Address> {
    /// There are enough encryption keys to generate a decryption key
    FinalizeKey { commitments: Vec<EncKeyCommitment<Signature, Address>>, current_session_id: SessionId },
    /// The session is over
    EndSession(SessionId),
}