use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{Config, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSessionId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub session_id: SessionId,
}

impl<C: Config> RpcParameter<C> for GetSessionId {
    type Response = Response;

    fn method() -> &'static str {
        "get_session_id"
    }

    async fn handler(self, _context: C) -> RpcResult<Self::Response> {
        let session_id = SessionId::get()?;
        Ok(Response { session_id })
    }
}
