use super::SubmitPartialKey;
use crate::primitives::*;
use dkg_primitives::{AppState, PartialKeyPayload, SessionId};
use serde::{Deserialize, Serialize};
use skde::key_generation::{generate_partial_key, PartialKey as SkdePartialKey};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestSubmitPartialKey {
    pub session_id: SessionId,
}

// RPC Method for committee members to submit their partial keys to the leader
impl<C: AppState> RpcParameter<C> for RequestSubmitPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "request_submit_partial_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        info!(
            "{} Submitted partial key to leader on session {:?}",
            context.log_prefix(),
            self.session_id
        );
        let (_, partial_key) = generate_partial_key(&context.skde_params()).unwrap();
        submit_partial_key_to_leader::<C>(self.session_id, partial_key, &context).await?;
        Ok(())
    }
}

pub async fn submit_partial_key_to_leader<C: AppState>(
    session_id: SessionId,
    partial_key: SkdePartialKey,
    ctx: &C,
) -> Result<(), RpcError> {
    if let Some(leader_rpc_url) = ctx.leader_rpc_url() {
        let payload = PartialKeyPayload::<C::Address>::new(ctx.address(), partial_key, session_id);
        let signature = ctx.sign(&payload).map_err(|e| RpcError::from(e))?;
        ctx.multicast(vec![leader_rpc_url], <SubmitPartialKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SubmitPartialKey { signature, payload });
        return Ok(());
    }
    Ok(())
}
