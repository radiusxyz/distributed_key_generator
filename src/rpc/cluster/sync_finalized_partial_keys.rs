use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{common::SyncFinalizedPartialKeysPayload, prelude::*},
    utils::{
        key::perform_randomized_aggregation, log::log_prefix_role_and_address,
        signature::verify_signature,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClusterSyncFinalizedPartialKeys {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

impl RpcParameter<AppState> for ClusterSyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;
        if &sender_address != &self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_role_and_address(context.config());
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        let SyncFinalizedPartialKeysPayload {
            partial_key_senders,
            partial_keys,
            session_id,
            ack_timestamp,
            signatures,
            ..
        } = &self.payload;

        info!(
            "{} Received finalized partial keys ACK - senders:{:?}, session_id: {:?
            }, timestamp: {}",
            prefix, partial_key_senders, session_id, ack_timestamp
        );

        // TODO: timestampes also should be collected assigned to each partial key
        if partial_key_senders.len() != partial_keys.len() || partial_keys.len() != signatures.len()
        {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                "Mismatched vector lengths in partial key ACK payload".into(),
            )));
        }

        for (sig, sender) in signatures.iter().zip(partial_key_senders) {
            let signer = verify_signature(sig, &self.payload)?;

            if &signer != sender {
                return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                    "Signature does not match partial key sender".into(),
                )));
            }
        }

        // TODO: Store this encryption key if signatures are valid and use for decryption key verification
        perform_randomized_aggregation(&context, *session_id, &partial_keys);

        Ok(())
    }
}
