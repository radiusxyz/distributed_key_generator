use radius_sdk::json_rpc::{
    client::{Id, RpcClient},
    server::{RpcError, RpcParameter},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{generate_partial_key, PartialKey as SkdePartialKey};
use tracing::info;

use super::{PartialKeyPayload, SubmitPartialKey};
use crate::{
    rpc::prelude::*,
    utils::{
        log::log_prefix_with_session_id, signature::create_signature, time::get_current_timestamp,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestSubmitPartialKey {
    pub session_id: SessionId,
}

// RPC Method for committee members to submit their partial keys to the leader
impl RpcParameter<AppState> for RequestSubmitPartialKey {
    type Response = Option<()>;

    fn method() -> &'static str {
        "request_submit_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_with_session_id(&context.config(), &self.session_id);
        let skde_params = context.skde_params();

        let (_, partial_key) = generate_partial_key(skde_params).unwrap();
        submit_partial_key_to_leader(self.session_id, partial_key, &context.clone()).await?;

        info!("{} Submitted partial key to leader", prefix);

        Ok(Some(()))
    }
}

pub async fn submit_partial_key_to_leader(
    session_id: SessionId,
    partial_key: SkdePartialKey,
    context: &AppState,
) -> Result<(), RpcError> {
    let leader_rpc_url = match context.config().is_leader() {
        true => context.config().cluster_rpc_url(),
        false => &context.config().leader_cluster_rpc_url().clone().unwrap(),
    };

    // Create payload with partial key and metadata
    let payload = PartialKeyPayload {
        sender: context.config().address().clone(),
        partial_key,
        submit_timestamp: get_current_timestamp(),
        session_id,
    };

    // Create signature for the payload
    let signature = create_signature(context, &payload).unwrap();

    let parameter = SubmitPartialKey { signature, payload };

    // Submit to leader
    let rpc_client = RpcClient::new()?;

    // Explicitly specify the type to prevent never type fallback issues
    let _: () = rpc_client
        .request::<SubmitPartialKey, ()>(
            &leader_rpc_url,
            SubmitPartialKey::method(),
            parameter,
            Id::Null,
        )
        .await?;

    Ok(())
}
