use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, AggregatedKey};

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKeyResponse {
    pub encryption_key: String,
}

impl<C: AppState> RpcParameter<C> for GetEncryptionKey {
    type Response = GetEncryptionKeyResponse;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        Ok(GetEncryptionKeyResponse { 
            encryption_key: AggregatedKey::get(self.session_id)?.enc_key().into() 
        })
    }
}
