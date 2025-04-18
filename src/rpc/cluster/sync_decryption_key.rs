use std::time::{SystemTime, UNIX_EPOCH};

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

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKey {
    pub signature: Signature,
    pub payload: SyncDecryptionKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKeyPayload {
    pub decryption_key: String,
    pub session_id: SessionId,
    pub solve_timestamp: u64,
    pub ack_solve_timestamp: u64,
}

impl RpcParameter<AppState> for SyncDecryptionKey {
    type Response = ();

    fn method() -> &'static str {
        "sync_decryption_key"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.payload, &_context)?;

        info!(
            "Received decryption key ACK - session_id: {:?}, timestamps: {} / {}",
            self.payload.session_id, self.payload.solve_timestamp, self.payload.ack_solve_timestamp
        );

        // Validation logic for decryption key is omitted (required in actual implementation)
        // Just recording the acknowledgment here

        // TODO: Add logic to store validator logs
        // (Required in actual implementation for validators to verify leader behavior)

        Ok(())
    }
}

// Broadcast decryption key acknowledgment from leader to the network
pub fn broadcast_decryption_key_ack(
    session_id: SessionId,
    decryption_key: String,
    solve_timestamp: u64,
    context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    let ack_solve_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let payload = SyncDecryptionKeyPayload {
        session_id,
        decryption_key,
        solve_timestamp,
        ack_solve_timestamp,
    };

    let signature = context
        .config()
        .signer()
        .sign_message(&serialize_to_bincode(&payload).unwrap())
        .unwrap();

    let parameter = SyncDecryptionKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    SyncDecryptionKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
