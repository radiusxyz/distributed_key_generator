use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSessionId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSessionIdResponse {
    pub session_id: SessionId,
}

impl<C: AppState> RpcParameter<C> for GetSessionId {
    type Response = GetSessionIdResponse;

    fn method() -> &'static str {
        "get_session_id"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;
        Ok(GetSessionIdResponse { session_id })
    }
}
