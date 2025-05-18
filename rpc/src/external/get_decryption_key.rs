use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, DecryptionKey};   

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub decryption_key: String,
}

impl<C> RpcParameter<C> for GetDecryptionKey
where
    C: AppState,
{
    type Response = GetDecryptionKeyResponse;

    fn method() -> &'static str {
        "get_decryption_key"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        Ok(GetDecryptionKeyResponse {
            decryption_key: DecryptionKey::get(self.session_id)?.into(),
        })
    }
}
