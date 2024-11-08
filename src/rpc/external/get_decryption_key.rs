use crate::rpc::prelude::*;

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKey {
    key_id: KeyId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecryptionKeyResponse {
    pub decryption_key: String,
}

impl GetDecryptionKey {
    pub const METHOD_NAME: &'static str = "get_decryption_key";

    pub async fn handler(
        parameter: RpcParameter,
        _context: Arc<AppState>,
    ) -> Result<GetDecryptionKeyResponse, RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let decryption_key = DecryptionKey::get(parameter.key_id)?;

        Ok(GetDecryptionKeyResponse {
            decryption_key: decryption_key.as_string(),
        })
    }
}
