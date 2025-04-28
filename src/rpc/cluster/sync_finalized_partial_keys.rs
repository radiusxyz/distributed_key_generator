use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;

use crate::{
    rpc::prelude::*,
    utils::{log_prefix_role_and_address, perform_randomized_aggregation},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeys {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub partial_key_submissions: Vec<PartialKeySubmission>,
    pub session_id: SessionId,
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
            partial_key_submissions,
            session_id,
            ack_timestamp,
        } = &self.payload;

        info!(
            "{} Received finalized partial keys ACK - partial_key_submissions.len(): {:?}, session_id: {:?
            }, timestamp: {}",
            prefix,
            partial_key_submissions.len(),
            session_id,
            ack_timestamp
        );

        // Put aggregated key for a Cluster member
        let partial_key_submissions =
            PartialKeyAddressList::get(*session_id)?.get_partial_key_list(*session_id)?;
        let partial_keys: Vec<SkdePartialKey> = partial_key_submissions
            .iter()
            .map(|partial_key_submission| partial_key_submission.payload.partial_key.clone())
            .collect();
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

        Ok(())
    }
}
