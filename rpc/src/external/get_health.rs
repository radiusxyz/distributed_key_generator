use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::AppState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetHealth;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub status: bool,
}

impl<C: AppState> RpcParameter<C> for GetHealth {
    type Response = Response;

    fn method() -> &'static str {
        "health"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        // TODO: Implement health check
        Ok(Response { status: true })
    }
}
