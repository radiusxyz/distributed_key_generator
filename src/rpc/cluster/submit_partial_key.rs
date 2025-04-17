use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{
        cluster::{broadcast_partial_key_ack, PartialKeyAckPayload, SubmitPartialKeyAck},
        common::{create_signature, verify_signature},
        prelude::*,
    },
    types::SessionId,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload {
    pub sender: Address,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKeyResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SubmitPartialKey {
    type Response = SubmitPartialKeyResponse;

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to verify actual signature
        let _ = verify_signature(&self.signature, &self.payload)?;
        let sender_address = self.payload.sender.clone();

        info!(
            "Received partial key - session_id: {:?}, sender: {}, timestamp: {}",
            self.payload.session_id,
            sender_address.as_hex_string(),
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        info!("Key generator list: {:?}", key_generator_list);
        if !key_generator_list.is_key_generator_in_cluster(&sender_address) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(
                sender_address.as_hex_string(),
            )));
        }

        // Verify partial key validity
        let is_valid = skde::key_generation::verify_partial_key_validity(
            context.skde_params(),
            self.payload.partial_key.clone(),
            self.payload.proof.clone(),
        )
        .unwrap();

        if !is_valid {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                format!("{:?}", self.payload.partial_key),
            )));
        }

        // Store the partial key
        let partial_key = PartialKey::new(self.payload.partial_key.clone());
        partial_key.put(self.payload.session_id, &sender_address)?;

        // TODO: handle appropriate paratial key index
        let _ = broadcast_partial_key_ack(
            sender_address,
            self.payload.session_id,
            self.payload.partial_key.clone(),
            self.payload.proof.clone(),
            self.payload.submit_timestamp,
            0,
            &context,
        );

        Ok(SubmitPartialKeyResponse { success: true })
    }
}
