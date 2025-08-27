use dkg_primitives::to_signed_commitment;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetHeartbeat;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetHeartbeatResponse<Signature, Address> {
    pub commitment: SignedCommitment<Signature, Address>,
}

impl<C: Config> RpcParameter<C> for GetHeartbeat {
    type Response = GetHeartbeatResponse<C::Signature, C::Address>;

    fn method() -> &'static str {
        "get_heartbeat"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let session_id = ctx
            .db_manager()
            .current_session()
            .map_err(|e| RpcError::from(e))?;
        let commitment = to_signed_commitment(
            ctx.clone(),
            session_id,
            HeartbeatPayload {
                address: ctx.address(),
                at: session_id,
            },
        )?;
        Ok(GetHeartbeatResponse { commitment })
    }
}
