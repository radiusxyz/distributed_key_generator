use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKey {
    key_id: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKeyResponse {
    pub encryption_key: String,
}

impl GetEncryptionKey {
    pub const METHOD_NAME: &'static str = "get_encryption_key";

    pub async fn handler(
        _: RpcParameter,
        context: Arc<AppState>,
    ) -> Result<GetEncryptionKeyResponse, RpcError> {
        let encryption_key = context.get_encryption_key().await;

        Ok(GetEncryptionKeyResponse { encryption_key })
    }
}
