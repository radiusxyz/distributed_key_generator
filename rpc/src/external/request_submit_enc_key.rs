use crate::*;
use dkg_primitives::{Config, SessionId, KeyService};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for submitted encryption key for given session
pub struct RequestSubmitEncKey {
    pub session_id: SessionId,
}

impl<C: Config> RpcParameter<C> for RequestSubmitEncKey {
    type Response = ();

    fn method() -> &'static str {
        "request_submit_enc_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        info!("method::{}", <Self as RpcParameter<C>>::method());
        let session_id = self.session_id;
        if !session_id.is_initial() { return Ok(()); } 
        let enc_key = ctx.key_service().gen_enc_key(ctx.randomness(session_id), None)?;
        submit_enc_key(&ctx, session_id, enc_key)?;
        Ok(())
    }
}
