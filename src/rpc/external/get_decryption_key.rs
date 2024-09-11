use skde::delay_encryption::SecretKey;

use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    key_id: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub decryption_key: SecretKey,
}

impl GetDecryptionKey {
    pub const METHOD_NAME: &'static str = "get_decryption_key";

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetDecryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let decryption_key = DecryptionKeyModel::get(parameter.key_id)?;

        Ok(GetDecryptionKeyResponse { decryption_key })
    }
}
