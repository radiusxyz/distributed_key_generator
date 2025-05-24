use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, EncKey, Error, SessionId};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKey;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKeyResponse {
    session_id: SessionId,
    key: Vec<u8>,
}

impl GetEncKeyResponse {
    pub fn new(session_id: SessionId, key: Vec<u8>) -> Self {
        Self { session_id, key }
    }
}

impl<C: AppState> RpcParameter<C> for GetEncKey {
    type Response = GetEncKeyResponse;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, _ctx: C) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;
        loop {
            if let Some(prev) = session_id.prev() {
                match EncKey::get(prev) {
                    Ok(enc_key) => return Ok(GetEncKeyResponse::new(session_id, enc_key.inner())),
                    Err(_) => continue,
                }
            } else {
                return Err(RpcError::from(Error::Arithmetic));
            }
        }
    }
}
