use std::fmt::Debug;

use dkg_primitives::{Config, DecKey, Payload, SignedCommitment, SolverEvent};
use dkg_utils::timestamp;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::*;

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
        let sender =
            ctx.verify_signature(&self.0.signature, &self.0.commitment, self.0.sender())?;
        let session_id = self.0.session_id();
        info!(
            "{} at session: {:?}",
            <Self as RpcParameter<C>>::method(),
            session_id
        );
        multicast_dec_key_ack::<C>(&ctx, self.payload(), session_id, vec![ctx.address()])?;
        // This is end of the session
        let dec_key = self.payload().decode::<DecKeyPayload>()?.dec_key;
        ctx.async_task()
            .emit_event(
                SolverEvent::SubmitDecKey {
                    session_id,
                    timestamp: timestamp(),
                    dec_key: dec_key.clone(),
                    who: sender,
                }
                .into(),
            )
            .await
            .map_err(|e| RpcError::from(e))?;
        DecKey::new(dec_key).put(session_id)?;

        Ok(Response(true))
    }
}
