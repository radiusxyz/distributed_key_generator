use crate::{
    cluster::broadcast_decryption_key_ack, 
    primitives::*
};
use dkg_primitives::{AppState, SubmitDecryptionKeyPayload};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SubmitDecryptionKeyPayload<Address>,
}

impl<Signature, Address> Display for SubmitDecryptionKey<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "🔑 Received decryption key at {:?} on session {:?}", self.payload.timestamp, self.payload.session_id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecryptionKeyResponse {
    pub success: bool,
}

impl<C: AppState> RpcParameter<C> for SubmitDecryptionKey<C::Signature, C::Address> {
    type Response = DecryptionKeyResponse;

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

        Ok(DecryptionKeyResponse { success: true })
    }
}
