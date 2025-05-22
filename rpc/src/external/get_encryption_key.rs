use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, Error, EncKeyFor, SecureBlock};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKey;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncKeyResponse<EncKey> {
    session_id: SessionId,
    key: EncKey,
}

impl<EncKey> GetEncKeyResponse<EncKey> {
    pub fn new(session_id: SessionId, key: EncKey) -> Self {
        Self { session_id, key }
    }
}

impl<C: AppState> RpcParameter<C> for GetEncKey {
    type Response = GetEncKeyResponse<EncKeyFor<C>>;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;
        loop {
            if let Some(prev) = session_id.prev() {
                match ctx.secure_block().get_enc_key(prev) {
                    Ok(enc_key) => return Ok(GetEncKeyResponse::new(session_id, enc_key)),
                    Err(_) => continue,
                }
            } else {
                return Err(RpcError::from(Error::Arithmetic));
            }
        }
    }
}
