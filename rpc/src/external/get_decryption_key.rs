use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{Config, SessionId, DecKey};   

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    key: Vec<u8>,
}

impl<C: Config> RpcParameter<C> for GetDecKey {
    type Response = Response;

    fn method() -> &'static str {
        "get_decryption_key"
    }

    async fn handler(self, _ctx: C) -> RpcResult<Self::Response> {
        let dec_key = DecKey::get(self.session_id)?;
        Ok(Response { key: dec_key.inner() })
    }
}
