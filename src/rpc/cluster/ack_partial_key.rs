use std::time::{SystemTime, UNIX_EPOCH};

use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};
use tracing::info;

use crate::rpc::{common::create_signature, prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedPartialKeyAck {
    pub signature: Signature,
    pub payload: PartialKeyAckPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyAckPayload {
    pub partial_key_sender: Address,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub index: usize,
    pub session_id: SessionId,
    pub submit_timestamp: u64,
    pub ack_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyAckResponse {
    pub success: bool,
}

impl RpcParameter<AppState> for SignedPartialKeyAck {
    type Response = PartialKeyAckResponse;

    fn method() -> &'static str {
        "ack_partial_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "Received partial key ACK - session_id: {}, index: {}, timestamp: {}",
            self.payload.session_id, self.payload.index, self.payload.ack_timestamp
        );

        // TODO: Leader verification (only leader can send ACK)

        // TODO: Store and process partial key index information
        // (In actual implementation, a structure to store index information is needed)

        Ok(PartialKeyAckResponse { success: true })
    }
}

// Broadcast partial key acknowledgment from leader to the entire network
pub fn broadcast_partial_key_ack(
    session_id: SessionId,
    partial_key: SkdePartialKey,
    proof: PartialKeyProof,
    submit_timestamp: u64,
    index: usize,
    context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let ack_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let payload = PartialKeyAckPayload {
        partial_key_sender: context.config().signer().address().clone(),
        session_id,
        partial_key,
        proof,
        index,
        submit_timestamp,
        ack_timestamp,
    };

    // TODO: Add to make actual signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

    let parameter = SignedPartialKeyAck { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    SignedPartialKeyAck::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
