use dkg_primitives::{Config, KeyGenerator, SessionId};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::*;

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
        if !session_id.is_initial() {
            return Ok(());
        }
        let randomness = ctx.randomness(session_id);
        let enc_key = ctx.key_generator().read().await.gen_enc_key(&randomness, None)?;
        submit_enc_key(ctx, session_id, enc_key)?;
        Ok(())
    }
}
