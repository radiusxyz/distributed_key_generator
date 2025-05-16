use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestSessionId {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestSessionIdResponse {
    pub latest_session_id: SessionId,
}

impl<C: AppState> RpcParameter<C> for GetLatestSessionId {
    type Response = GetLatestSessionIdResponse;

    fn method() -> &'static str {
        "get_latest_session_id"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        let latest_session_id = SessionId::get()?;
        Ok(GetLatestSessionIdResponse {
            latest_session_id,
        })
    }
}
