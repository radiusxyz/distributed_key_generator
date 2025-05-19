use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, AggregatedKey, Error};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKey;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKeyResponse {
    session_id: SessionId,
    encryption_key: String,
}

impl GetLatestEncryptionKeyResponse {
    pub fn new(session_id: SessionId, encryption_key: String) -> Self {
        Self { session_id, encryption_key }
    }
}

impl<C: AppState> RpcParameter<C> for GetLatestEncryptionKey {
    type Response = GetLatestEncryptionKeyResponse;

    fn method() -> &'static str {
        "get_latest_encryption_key"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;
        loop {
            if let Some(prev) = session_id.prev() {
                match AggregatedKey::get(prev) {
                    Ok(agg) => {
                        return Ok(GetLatestEncryptionKeyResponse::new(
                            session_id, 
                            agg.enc_key()
                        ));
                    }
                    Err(_) => {
                        continue;
                    }
                }
            } else {
                return Err(RpcError::from(Error::Arithmetic));
            }
        }
    }
}
