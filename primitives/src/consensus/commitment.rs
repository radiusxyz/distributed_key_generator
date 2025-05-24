use crate::SessionId;
use super::Payload;
use serde::{Deserialize, Serialize};
use dkg_utils::timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A commitment to a payload
pub struct Commitment<Address> {
    /// The payload of the commitment
    pub payload: Payload,
    /// The sender of the commitment
    pub sender: Option<Address>,
    /// The session ID of the commitment
    pub session_id: SessionId,
    /// The timestamp of the commitment
    pub timestamp: u128,
}

impl<Address> Commitment<Address> {
    /// Create a new instance of the commitment
    pub fn new(payload: Payload, sender: Option<Address>, session_id: SessionId) -> Self {
        Self { payload, sender, session_id, timestamp: timestamp() }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// A signed commitment
pub struct SignedCommitment<Signature, Address> {
    /// The commitment
    pub commitment: Commitment<Address>,
    /// The signature of the commitment
    pub signature: Signature,
}

impl<Signature, Address: std::fmt::Debug> std::fmt::Display for SignedCommitment<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "🔒 Commitment at {:?} on session {:?} from {:?}", self.commitment.timestamp, self.commitment.session_id, self.commitment.sender)
    }
}

impl<Signature, Address: Clone> SignedCommitment<Signature, Address> {
    pub fn session_id(&self) -> SessionId {
        self.commitment.session_id
    }

    pub fn timestamp(&self) -> u128 {
        self.commitment.timestamp
    }

    pub fn sender(&self) -> Option<Address> {
        self.commitment.clone().sender
    }
}

