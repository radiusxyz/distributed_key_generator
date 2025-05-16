use crate::{
    cluster::broadcast_decryption_key_ack, 
    primitives::*
};
use std::time::{SystemTime, UNIX_EPOCH};
use dkg_primitives::{AppState, KeyGenerationError, SessionId};
use dkg_utils::signature::verify_signature;
use radius_sdk::signature::{Address, Signature};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKey {
    pub signature: Signature,
    pub payload: SubmitDecryptionKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKeyPayload {
    pub sender: Address,
    pub decryption_key: String,
    pub session_id: SessionId,
    pub timestamp: u128,
}

impl SubmitDecryptionKeyPayload {
    pub fn new(sender: Address, decryption_key: String, session_id: SessionId) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, decryption_key, session_id, timestamp }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecryptionKeyResponse {
    pub success: bool,
}

impl<C> RpcParameter<C> for SubmitDecryptionKey
where
    C: AppState + 'static,
{
    type Response = DecryptionKeyResponse;

    fn method() -> &'static str {
        "submit_decryption_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();

        let sender_address = verify_signature(&self.signature, &self.payload)?;
        if sender_address != self.payload.sender {
            let err_msg = "Signature does not match sender address";
            error!("{} {}", prefix, err_msg);
            return Err(RpcError::from(KeyGenerationError::InternalError(
                err_msg.into(),
            )));
        }

        info!(
            "{} Received decryption key - session_id: {:?}, timestamp: {}",
            prefix, self.payload.session_id, self.payload.timestamp
        );

        broadcast_decryption_key_ack(
            self.payload.session_id,
            self.payload.decryption_key.clone(),
            self.payload.timestamp,
            &context,
        )?;

        Ok(DecryptionKeyResponse { success: true })
    }
}
