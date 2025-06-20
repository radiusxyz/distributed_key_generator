use crate::{EncKeyCommitment, SessionId};
use std::fmt::Debug;

/// Event of the runtime
#[derive(Clone)]
pub enum RuntimeEvent<Signature, Address> {
    /// There are enough encryption keys to generate a decryption key
    FinalizeKey { commitments: Vec<EncKeyCommitment<Signature, Address>>, start_session_id: SessionId },
    /// Solve the given encryption key and create a signed commitment
    SolveKey { enc_key: Vec<u8>, session_id: SessionId },
    /// The session is over
    EndSession(SessionId),
}

impl<Signature, Address> Debug for RuntimeEvent<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeEvent::FinalizeKey { start_session_id, .. } => write!(f, "🔒 Event::FinalizeKey at session {:?}", start_session_id),
            RuntimeEvent::SolveKey { session_id, .. } => write!(f, "🔑 Event::SolveKey at session {:?}", session_id), 
            RuntimeEvent::EndSession(session_id) => write!(f, "⏹️ Event::EndSession at session {:?}", session_id),
        }
    }
}