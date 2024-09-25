use skde::delay_encryption::PublicKey;

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKey {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestEncryptionKeyResponse {
    pub key_id: u64,
    pub encryption_key: PublicKey,
}

impl GetLatestEncryptionKey {
    pub const METHOD_NAME: &'static str = "get_latest_encryption_key";

    pub async fn handler(
        _parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetLatestEncryptionKeyResponse, RpcError> {
        let mut key_id = KeyIdModel::get()?;

        loop {
            if AggregatedKeyModel::get(key_id).is_err() {
                key_id -= 1;
                continue;
            }

            let aggregated_key = AggregatedKeyModel::get(key_id)?;
            let encryption_key = PublicKey {
                pk: aggregated_key.u.clone(),
            };

            return Ok(GetLatestEncryptionKeyResponse {
                key_id,
                encryption_key,
            });
        }
    }
}
