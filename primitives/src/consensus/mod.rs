mod commitment;
mod payload;

pub use commitment::*;
pub use payload::*;
use serde::Serialize;

use crate::{AddressFor, Config as ConfigT, ConfigErrorFor, SessionId, SignatureFor};

pub fn to_signed_commitment<C, Payload>(
    ctx: C,
    session_id: SessionId,
    payload: Payload,
) -> Result<SignedCommitment<SignatureFor<C>, AddressFor<C>>, ConfigErrorFor<C>>
where
    C: ConfigT,
    Payload: Serialize,
{
    let bytes = serde_json::to_vec(&payload)?;
    let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    Ok(SignedCommitment {
        signature,
        commitment,
    })
}

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
