use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    rpc::{common::verify_signature, prelude::*},
    utils::AddressExt,
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

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to verify actual signature
        let _sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "[{}] Received decryption key - session_id: {:?}, timestamp: {}",
            _context.config().address().to_short(),
            self.payload.session_id,
            self.payload.timestamp
        );

        // Store decryption key
        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());
        decryption_key.put(self.payload.session_id)?;

        // TODO: Add broadcast_decryption_key_ack

        info!(
            "[{}] Complete to get decryption key - key_id: {:?} / decryption key: {:?}",
            _context.config().address().to_short(),
            self.payload.session_id,
            decryption_key
        );

        Ok(DecryptionKeyResponse { success: true })
    }
}
