use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKey {
    key_id: KeyId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKeyResponse {
    pub encryption_key: String,
}

impl GetEncryptionKey {
    pub const METHOD_NAME: &'static str = "get_encryption_key";

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetEncryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let aggregated_key = AggregatedKey::get(parameter.key_id)?;
        let encryption_key = aggregated_key.encryption_key();

        Ok(GetEncryptionKeyResponse {
            encryption_key: encryption_key,
        })
    }
}
