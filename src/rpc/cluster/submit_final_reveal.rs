use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{SyncDecryptionKey, SyncPartialKey};
use crate::rpc::{common::create_signature, prelude::*};

// Message from leader to verifiers
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitFinalReveal {
    pub signature: Signature,
    pub payload: FinalRevealPayload,
}

// TODO: determine the structure of the payload
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload {
    pub session_id: SessionId,
    pub partial_keys: Vec<SyncPartialKey>,
    pub sync_decryption_key: SyncDecryptionKey,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevealResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SubmitFinalReveal {
    type Response = RevealResponse;

    fn method() -> &'static str {
        "submit_final_reveal"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        info!(
            "Received final reveal - session_id: {:?}, partial_keys: {}",
            self.payload.session_id,
            self.payload.partial_keys.len()
        );

        // Logic to store final reveal information
        // In actual implementation, validators store this data for later verification

        // TODO: Add validator logic (only logging for now)
        // Actual validators can use this data to verify fairness of leader decisions

        Ok(RevealResponse { success: true })
    }
}

// Broadcast final reveal information from leader
pub fn broadcast_final_reveal(
    session_id: SessionId,
    partial_keys: Vec<SyncPartialKey>,
    sync_decryption_key: SyncDecryptionKey,
    _context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let payload = FinalRevealPayload {
        session_id,
        partial_keys,
        sync_decryption_key,
    };

    // TODO: Add to make actual signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

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
