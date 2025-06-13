use crate::{EncKeyCommitment, SessionId};

/// Event of the runtime
#[derive(Debug, Clone)]
pub enum RuntimeEvent<Signature, Address> {
    /// There are enough encryption keys to generate a decryption key
    FinalizeKey { commitments: Vec<EncKeyCommitment<Signature, Address>>, start_session_id: SessionId },
    /// Solve the given encryption key and create a signed commitment
    SolveKey { enc_key: Vec<u8>, session_id: SessionId },
    /// The session is over
    EndSession(SessionId),
}