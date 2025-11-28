use dkg_primitives::{Config, EncKey, KeyGenerator, RuntimeError, SessionId};
use serde::{Deserialize, Serialize};

use crate::*;
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKey;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKeyResponse {
    session_id: SessionId,
    key: String,
}

impl GetEncKeyResponse {
    pub fn new(session_id: SessionId, key: String) -> Self {
        Self { session_id, key }
    }
}

impl<C: Config> RpcParameter<C> for GetEncKey {
    type Response = GetEncKeyResponse;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let session_id = SessionId::get()?;
        tracing::info!("Handling get_encryption_key request at {:?}", session_id);
        loop {
            if let Some(prev) = session_id.prev() {
                match EncKey::get(prev) {
                    Ok(enc_key) => {
                        let enc_key = ctx
                            .key_generator()
                            .read()
                            .await
                            .get_enc_key(&enc_key.inner())?;
                        tracing::info!("Encryption key retrieved successfully => {}", enc_key);
                        return Ok(GetEncKeyResponse::new(prev, enc_key));
                    }
                    Err(_) => continue,
                }
            } else {
                return Err(RpcError::from(RuntimeError::Arithmetic));
            }
        }
    }
}
