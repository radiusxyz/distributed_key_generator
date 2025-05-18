use crate::{
    cluster::broadcast_decryption_key_ack, 
    primitives::*
};
use dkg_primitives::{AppState, KeyGenerationError, SubmitDecryptionKeyPayload};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SubmitDecryptionKeyPayload<Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecryptionKeyResponse {
    pub success: bool,
}

impl<C> RpcParameter<C> for SubmitDecryptionKey<C::Signature, C::Address>
where
    C: AppState + 'static,
{
    type Response = DecryptionKeyResponse;

    fn method() -> &'static str {
        "submit_decryption_key"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();

        let _ = context.verify_signature(&self.signature, &self.payload, &self.payload.sender)?;
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
