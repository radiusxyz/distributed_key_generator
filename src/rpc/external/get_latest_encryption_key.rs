use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKey {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKeyResponse {
    pub key_id: KeyId,
    pub encryption_key: String,
}

impl RpcParameter<AppState> for GetLatestEncryptionKey {
    type Response = GetLatestEncryptionKeyResponse;

    fn method() -> &'static str {
        "get_latest_encryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let mut key_id = KeyId::get()?;

        loop {
            if AggregatedKey::get(key_id).is_err() {
                key_id.decrease_key_id();
                continue;
            }

            let aggregated_key = AggregatedKey::get(key_id)?;
            let encryption_key = aggregated_key.encryption_key();

            return Ok(GetLatestEncryptionKeyResponse {
                key_id,
                encryption_key,
            });
        }
    }
}
