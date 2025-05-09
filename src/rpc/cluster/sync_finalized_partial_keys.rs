use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};

use crate::{
    rpc::{
        common::{
            process_partial_key_submissions, validate_partial_key_submission,
            SyncFinalizedPartialKeysPayload,
        },
        prelude::*,
    },
    utils::{key::perform_randomized_aggregation, log::log_prefix_role_and_address},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClusterSyncFinalizedPartialKeys {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

impl RpcParameter<AppState> for ClusterSyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(context.config());

        validate_partial_key_submission(&self.signature, &self.payload)?;

        let partial_keys = process_partial_key_submissions(&prefix, &self.payload)?;

        perform_randomized_aggregation(&context, self.payload.session_id, &partial_keys);

        Ok(())
    }
}
