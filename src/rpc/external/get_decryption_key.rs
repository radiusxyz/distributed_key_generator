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
        _context: Arc<AppState>,
    ) -> Result<GetDecryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let aggregated_key = AggregatedKeyModel::get(parameter.key_id)?;
        let encryption_key = PublicKey {
            pk: aggregated_key.u.clone(),
        };

        Ok(GetDecryptionKeyResponse { encryption_key })
    }
}
