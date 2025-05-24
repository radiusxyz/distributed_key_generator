use super::SubmitEncKey;
use crate::primitives::*;
use dkg_primitives::{AppState, SessionId, AsyncTask, SecureBlock, Commitment, SignedCommitment};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestSubmitEncKey {
    pub session_id: SessionId,
}

// RPC Method for committee members to submit their partial keys to the leader
impl<C: AppState> RpcParameter<C> for RequestSubmitEncKey {
    type Response = ();

    fn method() -> &'static str {
        "request_submit_enc_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let session_id = self.session_id;
        info!("Submitted partial key to leader on session {:?}", session_id);
        let enc_key = ctx.secure_block().gen_enc_key(ctx.randomness(session_id), None)?;
        submit_enc_key::<C>(session_id, enc_key, &ctx).await?;
        Ok(())
    }
}

pub async fn submit_enc_key<C: AppState>(
    session_id: SessionId,
    enc_key: Vec<u8>,
    ctx: &C,
) -> Result<(), RpcError> {
    if let Some(leader_rpc_url) = ctx.leader_rpc_url() {
        let commitment = Commitment::new(enc_key.into(), Some(ctx.address()), session_id);
        let signature = ctx.sign(&commitment).map_err(|e| RpcError::from(e))?;
        ctx.async_task().multicast(vec![leader_rpc_url], <SubmitEncKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(), SubmitEncKey(SignedCommitment { commitment, signature }));
        return Ok(());
    }
    Ok(())
}
