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

use super::{SignedDecryptionKeyAck, SubmitPartialKeyAck};
use crate::rpc::{common::create_signature, prelude::*};

// Message from leader to verifiers
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedFinalReveal {
    pub signature: Signature,
    pub payload: FinalRevealPayload,
}

// TODO: determine the structure of the payload
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload {
    pub session_id: SessionId,
    pub partial_keys: Vec<SubmitPartialKeyAck>,
    pub decryption_ack: SignedDecryptionKeyAck,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RevealResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SignedFinalReveal {
    type Response = RevealResponse;

    fn method() -> &'static str {
        "final_reveal"
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
    partial_keys: Vec<SubmitPartialKeyAck>,
    decryption_ack: SignedDecryptionKeyAck,
    _context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let payload = FinalRevealPayload {
        session_id,
        partial_keys,
        decryption_ack,
    };

    // TODO: Add to make actual signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

    let parameter = SignedFinalReveal { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    SignedFinalReveal::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
