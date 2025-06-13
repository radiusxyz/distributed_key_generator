use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{Config, EncKey, RuntimeError, SessionId};
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

impl<C: Config> RpcParameter<C> for GetEncKey {
    type Response = GetEncKeyResponse;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, _ctx: C) -> RpcResult<Self::Response> {
        let session_id = SessionId::get()?;
        loop {
            if let Some(prev) = session_id.prev() {
                match EncKey::get(prev) {
                    Ok(enc_key) => return Ok(GetEncKeyResponse::new(session_id, enc_key.inner())),
                    Err(_) => continue,
                }
            } else {
                return Err(RpcError::from(RuntimeError::Arithmetic));
            }
        }
    }
}
