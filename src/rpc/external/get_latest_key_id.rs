use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestKeyId {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetLatestKeyIdResponse {
    pub latest_session_id: SessionId,
}

impl RpcParameter<AppState> for GetLatestKeyId {
    type Response = GetLatestKeyIdResponse;

    fn method() -> &'static str {
        "get_latest_key_id"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let session_id = SessionId::get()?;

        // loop {
        return Ok(GetLatestKeyIdResponse {
            latest_session_id: session_id,
        });
        // }
    }
}
