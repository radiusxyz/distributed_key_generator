use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;
use dkg_types::error::KeyGenerationError;
use dkg_utils::{
    key::perform_randomized_aggregation,
    log::log_prefix_role_and_address,
    signature::verify_signature,
};
use crate::{
    primitives::*,
    common::SyncFinalizedPartialKeysPayload,
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
        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_role_and_address(context.config());

        let SyncFinalizedPartialKeysPayload {
            sender,
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
        for (i, pk_submission) in self.payload.partial_key_submissions.iter().enumerate() {
            let signable_message = pk_submission.payload.clone();

            let signer = verify_signature(&pk_submission.signature, &signable_message)?;

            if signer != pk_submission.payload.sender {
                return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                    format!(
                        "[Cluster] Signature mismatch at index {}: expected {:?}, got {:?}",
                        i, sender, signer
                    ),
                )));
            }

            PartialKeyAddressList::apply(self.payload.session_id, |list| {
                list.insert(sender.clone());
            })?;

            PartialKeySubmission::new(pk_submission).put(self.payload.session_id, sender)?;
        }

        // TODO: Store this encryption key if signatures are valid and use for decryption key verification
        perform_randomized_aggregation(&context, *session_id, &partial_keys);

        Ok(())
    }
}
