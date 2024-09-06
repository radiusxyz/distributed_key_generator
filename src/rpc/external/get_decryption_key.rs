use skde::delay_encryption::PublicKey;

use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    key_id: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub encryption_key: PublicKey,
}

impl GetDecryptionKey {
    pub const METHOD_NAME: &'static str = "get_encryption_key";

    pub async fn handler(
        parameter: RpcParameter,
        context: Arc<AppState>,
    ) -> Result<GetDecryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let aggregated_key = context.get_encryption_key(parameter.key_id).await?;

        let encryption_key = PublicKey {
            pk: aggregated_key.u.clone(),
        };

        Ok(GetDecryptionKeyResponse { encryption_key })
    }
}
