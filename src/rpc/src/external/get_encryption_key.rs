use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKey {
    session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKeyResponse {
    pub encryption_key: String,
}

impl RpcParameter<AppState> for GetEncryptionKey {
    type Response = GetEncryptionKeyResponse;

    fn method() -> &'static str {
        "get_encryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let encryption_key = AggregatedKey::get(self.session_id)?.enc_key();
        Ok(GetEncryptionKeyResponse { encryption_key })
    }
}
