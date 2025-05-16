use crate::{
    primitives::*, 
    common::{
        process_partial_key_submissions
    },
};
use dkg_primitives::{AppState, SyncFinalizedPartialKeysPayload, KeyGenerationError};
use dkg_utils::key::perform_randomized_aggregation;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClusterSyncFinalizedPartialKeys<Signature> {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

impl<C: AppState> RpcParameter<C> for ClusterSyncFinalizedPartialKeys<C::Signature> 
where
    C::Signature: Send + 'static,
{
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();
        let sender_address = context.verify_signature(&self.signature, &self.payload)?;
        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }
        let partial_keys = process_partial_key_submissions(&prefix, &self.payload)?;

        perform_randomized_aggregation(&context, self.payload.session_id, &partial_keys);

        Ok(())
    }
}
