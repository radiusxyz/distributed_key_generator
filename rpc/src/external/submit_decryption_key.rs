use crate::{
    cluster::broadcast_decryption_key_ack, 
    primitives::*
};
use dkg_primitives::{AppState, SubmitDecKeyPayload};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SubmitDecKeyPayload<Address>,
}

impl<Signature, Address> Display for SubmitDecKey<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "🔑 Received decryption key at {:?} on session {:?}", self.payload.timestamp, self.payload.session_id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub success: bool,
}

impl<C: AppState> RpcParameter<C> for SubmitDecKey<C::Signature, C::Address> {
    type Response = Response;

    fn method() -> &'static str {
        "submit_decryption_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {

        let _ = context.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender))?;
        info!("{}", self);

        broadcast_decryption_key_ack::<C>(
            self.payload.session_id,
            self.payload.decryption_key.clone(),
            self.payload.timestamp,
            &context,
        )?;

        Ok(Response { success: true })
    }
}
