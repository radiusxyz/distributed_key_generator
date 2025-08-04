use crate::*;
use dkg_primitives::{Config, SessionEvent, Payload, SignedCommitment, DecKey};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fmt::Debug;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address> SubmitDecKey<Signature, Address> {
    pub fn payload(&self) -> Payload {
        self.0.commitment.payload.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response(pub bool);

impl<C: Config> RpcParameter<C> for SubmitDecKey<C::Signature, C::Address> {
    type Response = Response;

    fn method() -> &'static str {
        "submit_dec_key"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let _ = ctx.verify_signature(&self.0.signature, &self.0.commitment, self.0.sender())?;
        let session_id = self.0.session_id();
        info!("{} at session: {:?}", <Self as RpcParameter<C>>::method(), session_id);
        multicast_dec_key_ack::<C>(&ctx, self.payload(), session_id, vec![ctx.address()])?;
        // This is end of the session
        ctx.async_task().emit_event(SessionEvent::EndSession(session_id.into())).await.map_err(|e| RpcError::from(e))?;
        let dec_key  = self.payload().decode::<DecKeyPayload>()?.dec_key;
        DecKey::new(dec_key).put(session_id)?;

        Ok(Response(true))
    }
}
