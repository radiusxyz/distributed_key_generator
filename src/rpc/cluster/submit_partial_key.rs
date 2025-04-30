use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{cluster::broadcast_partial_key_ack, common::PartialKeyPayload, prelude::*},
    utils::{
        log::{log_prefix_with_session_id, AddressExt},
        signature::verify_signature,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKey {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

impl RpcParameter<AppState> for SubmitPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;

        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_with_session_id(context.config(), &self.payload.session_id);

        info!(
            "{} Received partial key - session_id: {:?}, sender: {}, timestamp: {}",
            prefix,
            self.payload.session_id.as_u64(),
            self.payload.sender.to_short(),
            self.payload.submit_timestamp
        );

        // Check if key generator is registered in the cluster
        let key_generator_list = KeyGeneratorList::get()?;
        if !key_generator_list.is_key_generator_in_cluster(&self.payload.sender) {
            return Err(RpcError::from(KeyGenerationError::NotRegisteredGenerator(
                self.payload.sender.as_hex_string(),
            )));
        }

        PartialKeyAddressList::initialize(self.payload.session_id)?;

        // if the sender is incluided in
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            // TODO: Should fix RACE condition
            info!("{} Inserted partial key into list: {:?} ", prefix, list);
            list.insert(self.payload.sender.clone());
        })?;

        let partial_key = PartialKey::new(self.payload.partial_key.clone());
        partial_key.put(self.payload.session_id, &self.payload.sender)?;

        let _ = broadcast_partial_key_ack(
            self.payload.sender,
            self.payload.session_id,
            self.payload.partial_key,
            self.payload.submit_timestamp,
            0,
            &context,
        );

        Ok(())
    }
}
