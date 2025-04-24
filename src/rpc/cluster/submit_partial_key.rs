use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{cluster::broadcast_partial_key_ack, common::verify_signature, prelude::*},
    types::SessionId,
    utils::AddressExt,
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
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}

impl RpcParameter<AppState> for SubmitPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // TODO: Add to verify actual signature
        let _ = verify_signature(&self.signature, &self.payload)?;
        let sender_address = self.payload.sender.clone();

        info!(
            "[{}] Received partial key - session_id: {:?}, sender: {}, timestamp: {}",
            context.config().address().to_short(),
            self.payload.session_id,
            sender_address.as_hex_string(),
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        if !key_generator_list.is_key_generator_in_cluster(&sender_address) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(
                sender_address.as_hex_string(),
            )));
        }

        PartialKeyAddressList::initialize(self.payload.session_id)?;

        // if the sender is incluided in
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(self.payload.sender.clone());
        })?;

        let partial_key = PartialKey::new(self.payload.partial_key.clone());
        partial_key.put(self.payload.session_id, &self.payload.sender)?;

        // TODO: handle appropriate paratial key index, (session_id, key_index in the session)
        let _ = broadcast_partial_key_ack(
            sender_address,
            self.payload.session_id,
            self.payload.partial_key,
            self.payload.submit_timestamp,
            0,
            &context,
        );

        Ok(())
    }
}
