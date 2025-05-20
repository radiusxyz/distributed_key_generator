use crate::{
    cluster::broadcast_decryption_key_ack, 
    primitives::*
};
use dkg_primitives::{AppState, SubmitDecryptionKeyPayload};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SubmitDecryptionKeyPayload<Address>,
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
        let prefix = context.log_prefix();

        let _ = context.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender))?;
        info!(
            "{} Received decryption key - session_id: {:?}, timestamp: {}",
            prefix, self.payload.session_id, self.payload.timestamp
        );

        broadcast_decryption_key_ack::<C>(
            self.payload.session_id,
            self.payload.decryption_key.clone(),
            self.payload.timestamp,
            &context,
        )?;

        Ok(DecryptionKeyResponse { success: true })
    }
}
