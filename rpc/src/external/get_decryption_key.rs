use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId, DecKey};   

/// 09/05
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetDecKey {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    key: String,
}

impl<C: AppState> RpcParameter<C> for GetDecKey {
    type Response = Response;

    fn method() -> &'static str {
        "get_decryption_key"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        Ok(Response { key: DecKey::get(self.session_id)?.into() })
    }
}
