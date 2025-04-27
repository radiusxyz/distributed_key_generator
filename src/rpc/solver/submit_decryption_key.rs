use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    rpc::{cluster::broadcast_decryption_key_ack, prelude::*},
    utils::{log_prefix_role_and_address, verify_signature},
};

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
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecryptionKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SubmitDecryptionKey {
    type Response = DecryptionKeyResponse;

    fn method() -> &'static str {
        "submit_decryption_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
        // TODO: Add to verify actual signature
        let _sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "{} Received decryption key - session_id: {:?}, timestamp: {}",
            prefix, self.payload.session_id, self.payload.timestamp
        );

        // Store decryption key
        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());
        decryption_key.put(self.payload.session_id)?;

        broadcast_decryption_key_ack(
            self.payload.session_id,
            self.payload.decryption_key.clone(),
            self.payload.timestamp,
            &context,
        )?;

        info!(
            "{} Complete to get decryption key - key_id: {:?} / decryption key: {:?}",
            prefix, self.payload.session_id, decryption_key
        );

        Ok(DecryptionKeyResponse { success: true })
    }
}
