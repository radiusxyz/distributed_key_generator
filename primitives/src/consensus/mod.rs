
mod commitment;
mod payload; 

pub use commitment::*;
pub use payload::*;

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("Invalid commitment: {0}")]
    InvalidCommitment(String),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
    #[error("Deserialize error: {0}")]
    DeserializeError(String),
}