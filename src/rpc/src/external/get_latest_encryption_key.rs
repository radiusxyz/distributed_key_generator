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
        let session_id = SessionId::get()?;
        loop {
            if let Some(prev) = session_id.prev() {
                match AggregatedKey::get(prev) {
                    Ok(agg) => {
                        let encryption_key = agg.enc_key();
            
                        return Ok(GetLatestEncryptionKeyResponse {
                            session_id,
                            encryption_key,
                        });
                    }
                    Err(_) => {
                        continue;
                    }
                }
            } else {
                // underflow
                return Err(RpcError::from(Error::Arithmetic));
            }
        }
    }
}
