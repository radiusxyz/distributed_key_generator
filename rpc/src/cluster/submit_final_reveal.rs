use super::{SyncDecryptionKey, SyncPartialKey};
use crate::primitives::*;
use dkg_primitives::{AppState, SessionId, Error, KeyGeneratorList};
use serde::{Deserialize, Serialize};
use tracing::info;

// TODO: Add handler to submit partial keys and decryption key from leader to a verifier
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitFinalReveal<Signature, Address> {
    pub signature: Signature,
    pub payload: FinalRevealPayload<Signature, Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload<Signature, Address> {
    pub session_id: SessionId,
    pub partial_keys: Vec<SyncPartialKey<Signature, Address>>,
    pub sync_decryption_key: SyncDecryptionKey<Signature, Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevealResponse {
    pub success: bool,
}

impl<C: AppState> RpcParameter<C> for SubmitFinalReveal<C::Signature, C::Address> {
    type Response = RevealResponse;

    fn method() -> &'static str {
        "submit_final_reveal"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();
        info!(
            "{} Received final reveal - session_id: {:?}, partial_keys: {}",
            prefix,
            self.payload.session_id,
            self.payload.partial_keys.len()
        );

        Ok(RevealResponse { success: true })
    }
}

// Broadcast final reveal information from leader
pub fn broadcast_final_reveal<C: AppState>(
    session_id: SessionId,
    partial_keys: Vec<SyncPartialKey<C::Signature, C::Address>>,
    sync_decryption_key: SyncDecryptionKey,
    context: &C,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let payload = FinalRevealPayload {
        session_id,
        partial_keys,
        sync_decryption_key,
    };

    let signature = context.create_signature(&payload).unwrap();

    let parameter = SubmitFinalReveal { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    SubmitFinalReveal::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
