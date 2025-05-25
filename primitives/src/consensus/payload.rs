use super::ConsensusError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A payload of a consensus message
pub struct Payload(Vec<u8>);

impl Payload {
    /// Create a new instance of the payload
    pub fn new(payload: Vec<u8>) -> Self {
        Self(payload)
    }

    /// Decode the payload into a specific type `T`
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, ConsensusError> {
        serde_json::from_slice(&self.0).map_err(|_| ConsensusError::InvalidPayload("Invalid payload".to_string()))
    }
}

impl From<Vec<u8>> for Payload {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}
