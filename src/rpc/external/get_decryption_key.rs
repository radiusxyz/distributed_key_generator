use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub decryption_key: String,
}

impl RpcParameter<AppState> for GetDecryptionKey {
    type Response = GetDecryptionKeyResponse;

    fn method() -> &'static str {
        "get_decryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let decryption_key = DecryptionKey::get(self.session_id)?;

        Ok(GetDecryptionKeyResponse {
            decryption_key: decryption_key.as_string(),
        })
    }
}
