use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::AppState;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Handler for submitting heartbeat
pub struct SubmitHeartbeat;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub status: bool,
}

impl<C: AppState> RpcParameter<C> for SubmitHeartbeat {
    type Response = Response;

    fn method() -> &'static str {
        "submit_heartbeat"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        Ok(Response { status: true })
    }
}
