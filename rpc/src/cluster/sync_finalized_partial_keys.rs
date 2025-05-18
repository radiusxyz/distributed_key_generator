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
pub struct ClusterSyncFinalizedPartialKeys<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload<Signature, Address>,
}

impl<C> RpcParameter<C> for ClusterSyncFinalizedPartialKeys<C::Signature, C::Address> 
where
    C: AppState
{
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let _ = context.verify_signature(&self.signature, &self.payload, &self.payload.sender)?;
        let partial_keys: Vec<skde::key_generation::PartialKey> = process_partial_key_submissions::<C>(&context, &self.payload)?;

        perform_randomized_aggregation(&context, self.payload.session_id, &partial_keys);

        Ok(())
    }
}
