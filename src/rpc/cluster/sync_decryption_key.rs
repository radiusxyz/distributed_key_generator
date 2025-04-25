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

use crate::{
    rpc::prelude::*,
    utils::{get_current_timestamp, AddressExt},
};

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

        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());
        decryption_key.put(self.payload.session_id)?;

        info!(
            "[{}, {}] Complete put decryption key - key_id: {:?} / decryption key: {:?}",
            _context.role(),
            _context.config().address().to_short(),
            self.payload.session_id,
            decryption_key
        );

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
    let ack_solve_timestamp = get_current_timestamp();
    info!(
        "[{}] Broadcast decryption key acknowledgment - session_id: {:?}, timestamps: {} / {}",
        context.config().address().to_short(),
        session_id,
        solve_timestamp,
        ack_solve_timestamp
    );

    let other_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_other_key_generator_rpc_url_list(context.config().address());

    let ack_solve_timestamp = get_current_timestamp();

    let payload = SyncDecryptionKeyPayload {
        session_id,
        decryption_key,
        solve_timestamp,
        ack_solve_timestamp,
    };

    let signature = context
        .config()
        .signer()
        .sign_message(serialize_to_bincode(&payload).unwrap())
        .unwrap();

    let parameter = SyncDecryptionKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    other_key_generator_rpc_url_list,
                    SyncDecryptionKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
