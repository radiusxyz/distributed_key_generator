use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::{
    rpc::{common::verify_signature, prelude::*},
    types::{KeyId, SessionId},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedPartialKey {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload {
    pub sender: Address,
    pub key_id: KeyId,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SignedPartialKey {
    type Response = PartialKeyResponse;

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to verify actual signature
        let sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "Received partial key - session_id: {}, key_id: {:?}, sender: {}, timestamp: {}",
            self.payload.session_id,
            self.payload.key_id,
            sender_address.as_hex_string(),
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        // if !KeyGeneratorList::get()?.is_key_generator_in_cluster(&sender_address) {
        //     return Err(RpcError {
        //         code: -32603,
        //         message: format!(
        //             "Address {} is not a registered key generator",
        //             sender_address.as_hex_string()
        //         )
        //         .into(),
        //         data: None,
        //     });
        // }

        // Verify partial key validity
        let _is_valid = skde::key_generation::verify_partial_key_validity(
            context.skde_params(),
            self.payload.partial_key.clone(),
            self.payload.proof,
        );

        // if !is_valid {
        //     return Err(RpcError {
        //         code: -32603,
        //         message: "Invalid partial key".into(),
        //         data: None,
        //     });
        // }

        // Initialize partial key address list for this key ID
        PartialKeyAddressList::initialize(self.payload.key_id)?;

        // Add sender address to the partial key address list
        PartialKeyAddressList::apply(self.payload.key_id, |list| {
            list.insert(sender_address.clone());
        })?;

        // Store the partial key
        let partial_key = PartialKey::new(self.payload.partial_key.clone());
        partial_key.put(self.payload.key_id, &sender_address)?;

        // TODO: Generate and broadcast ACK message (to be implemented in ack_partial_key method)

        Ok(PartialKeyResponse { success: true })
    }
}
