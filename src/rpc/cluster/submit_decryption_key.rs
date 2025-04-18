use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::rpc::{common::verify_signature, prelude::*};

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
        let sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "Received decryption key - session_id: {:?}, sender: {}, timestamp: {}",
            self.payload.session_id,
            sender_address.as_hex_string(),
            self.payload.timestamp
        );

        // Validation logic - in this example, only checking if Solver is registered in the cluster
        // if !KeyGeneratorList::get()?.is_key_generator_in_cluster(&sender_address) {
        //     return Err(RpcError::InvalidParams(format!(
        //         "Address {} is not a registered key generator",
        //         sender_address.as_hex_string()
        //     )));
        // }

        // Store decryption key
        let _decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());
        // decryption_key.put(self.payload.key_id)?;

        // TODO: Add broadcast logic via ack_decryption_key at this point
        // This will be implemented in a separate function

        Ok(DecryptionKeyResponse { success: true })
    }
}
