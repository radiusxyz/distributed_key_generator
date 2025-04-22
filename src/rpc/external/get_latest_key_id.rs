use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestSessionId {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestSessionIdResponse {
    pub latest_session_id: SessionId,
}

impl RpcParameter<AppState> for GetLatestSessionId {
    type Response = GetLatestSessionIdResponse;

    fn method() -> &'static str {
        "get_latest_session_id"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;

        // loop {
        return Ok(GetLatestSessionIdResponse {
            latest_session_id: session_id,
        });
        // }
    }
}
