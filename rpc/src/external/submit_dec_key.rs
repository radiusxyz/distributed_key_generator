use crate::*;
use dkg_primitives::{AppState, SignedCommitment, Payload};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fmt::{Debug, Display};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecKey<Signature, Address>(pub SignedCommitment<Signature, Address>);

impl<Signature, Address: Debug> Display for SubmitDecKey<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubmitDecKey: {}", self.0)
    }
}

impl<Signature, Address> SubmitDecKey<Signature, Address> {
    pub fn payload(&self) -> Payload {
        self.0.commitment.payload.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response(pub bool);

impl<C: AppState> RpcParameter<C> for SubmitDecKey<C::Signature, C::Address> {
    type Response = Response;

    fn method() -> &'static str {
        "submit_dec_key"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let _ = ctx.verify_signature(&self.0.signature, &self.0.commitment, self.0.sender())?;
        info!("{}", self);
        multicast_dec_key_ack::<C>(&ctx, self.payload(), self.0.session_id())?;
        Ok(Response(true))
    }
}
