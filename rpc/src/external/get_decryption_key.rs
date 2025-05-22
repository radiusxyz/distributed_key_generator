use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, DecKeyFor, SecureBlock};   

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response<DecKey> {
    key: DecKey,
}

impl<C: AppState> RpcParameter<C> for GetDecKey {
    type Response = Response<DecKeyFor<C>>;

    fn method() -> &'static str {
        "get_decryption_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        Ok(Response { key: ctx.secure_block().get_dec_key(self.session_id)? })
    }
}
