use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKey {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKeyResponse {
    pub session_id: SessionId,
    pub encryption_key: String,
}

impl RpcParameter<AppState> for GetLatestEncryptionKey {
    type Response = GetLatestEncryptionKeyResponse;

    fn method() -> &'static str {
        "get_latest_encryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let mut session_id = SessionId::get()?;

        loop {
            if AggregatedKey::get(session_id).is_err() {
                session_id.decrease_key_id();
                continue;
            }

            let aggregated_key = AggregatedKey::get(session_id)?;
            let encryption_key = aggregated_key.encryption_key();

            return Ok(GetLatestEncryptionKeyResponse {
                session_id,
                encryption_key,
            });
        }
    }
}
