
use crate::*;
use dkg_primitives::SessionEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncStartTime<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address> SyncStartTime<Signature, Address> {
    fn start_time(&self) -> Result<u128, RpcError> {
        let payload = self.0.commitment.payload.decode::<StartTimePayload>().map_err(|e| RpcError::from(e))?;
        Ok(payload.start_time())
    }
}

impl<C: Config> RpcParameter<C> for SyncStartTime<C::Signature, C::Address> {
    type Response = Option<u128>;

    fn method() -> &'static str {
        "sync_start_time"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        info!("method::{}", <Self as RpcParameter<C>>::method());
        if let Some(sender) = self.0.sender() {
            let session_id = self.0.session_id();
            info!("{:?} at session {:?}", <Self as RpcParameter<C>>::method(), session_id);
            let maybe_leader = ctx.verify_signature(&self.0.signature, &self.0.commitment, Some(sender))?;
            let (leader_address, _) = ctx.current_leader(session_id, false).map_err(|e| RpcError::from(e))?;
            if maybe_leader != leader_address {
                return Ok(None); 
            }
            if !session_id.is_initial() {
                return Ok(None);
            }
            let start_time = self.start_time()?;
            ctx.async_task().emit_event(SessionEvent::GenesisSession(start_time).into()).await?;
            return Ok(Some(start_time));   
        } 
        Ok(None)
    }
}