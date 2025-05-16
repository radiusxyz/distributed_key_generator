use super::SubmitPartialKey;
use crate::primitives::*;
use dkg_primitives::{AppState, SessionId, PartialKeyPayload};
use serde::{Deserialize, Serialize};
use skde::key_generation::{generate_partial_key, PartialKey as SkdePartialKey};
use tracing::info;


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestSubmitPartialKey {
    pub session_id: SessionId,
}

// RPC Method for committee members to submit their partial keys to the leader
impl<C: AppState> RpcParameter<C> for RequestSubmitPartialKey 
where
    C: 'static
{
    type Response = Option<()>;

    fn method() -> &'static str {
        "request_submit_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        info!("{} Submitted partial key to leader on session {:?}", context.log_prefix(), self.session_id);
        let (_, partial_key) = generate_partial_key(&context.skde_params()).unwrap();
        submit_partial_key_to_leader(self.session_id, partial_key, &context.clone()).await?;

        Ok(Some(()))
    }
}

pub async fn submit_partial_key_to_leader<C: AppState>(
    session_id: SessionId,
    partial_key: SkdePartialKey,
    context: &C,
) -> Result<(), RpcError> {
    let leader_rpc_url = context.leader_rpc_url();

    // Create payload with partial key and metadata
    let payload = PartialKeyPayload::new(context.address().clone(), partial_key, session_id);

    // Create signature for the payload
    let signature = context.create_signature(&payload).unwrap();

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
