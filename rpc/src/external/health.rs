use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::AppState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetHealth;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetHealthResponse {
    pub status: String,
}

impl<C: AppState> RpcParameter<C> for GetHealth {
    type Response = GetHealthResponse;

    fn method() -> &'static str {
        "health"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        Ok(GetHealthResponse { status: "ok".to_string() })
    }
}
