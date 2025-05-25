use crate::*;
use dkg_primitives::{AppState, SessionId, SecureBlock};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for submitted encryption key for given session
pub struct RequestSubmitEncKey {
    pub session_id: SessionId,
}

impl<C: AppState> RpcParameter<C> for RequestSubmitEncKey {
    type Response = ();

    fn method() -> &'static str {
        "request_submit_enc_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let session_id = self.session_id;
        info!("Generate enc key for session {:?}", session_id);
        let enc_key = ctx.secure_block().gen_enc_key(ctx.randomness(session_id), None)?;
        submit_enc_key::<C>(session_id, enc_key, &ctx).await?;
        Ok(())
    }
}
