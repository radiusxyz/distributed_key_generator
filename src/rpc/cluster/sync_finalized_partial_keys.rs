use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::prelude::*,
    utils::{log::log_prefix_role_and_address, key::perform_randomized_aggregation},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeys {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub partial_key_senders: Vec<Address>,
    pub partial_keys: Vec<SkdePartialKey>,
    pub session_id: SessionId,
    pub submit_timestamps: Vec<u64>,
    pub signatures: Vec<Signature>,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
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

        // Put aggregated key for a Cluster member
        let partial_keys =
            PartialKeyAddressList::get(*session_id)?.get_partial_key_list(*session_id)?;
        perform_randomized_aggregation(&context, *session_id, &partial_keys);

        // TODO: Signature verification
        // for (sig, sender) in signatures.iter().zip(partial_key_senders) {
        //     let signer = verify_signature(sig, &self.payload)?;

        //     if &signer != sender {
        //         return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
        //             "Signature does not match sender".into(),
        //         )));
        //     }
        // }

        // TODO: Calculate and store encryption key if signatures are valid

        Ok(())
    }
}
