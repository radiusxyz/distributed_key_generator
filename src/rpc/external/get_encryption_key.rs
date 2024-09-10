use skde::delay_encryption::SecretKey;

use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKey {
    key_id: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetEncryptionKeyResponse {
    pub decryption_key: SecretKey,
}

impl GetEncryptionKey {
    pub const METHOD_NAME: &'static str = "get_decryption_key";

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetEncryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let decryption_key = DecryptionKeyModel::get(parameter.key_id)?;

        Ok(GetEncryptionKeyResponse { decryption_key })
    }
}
