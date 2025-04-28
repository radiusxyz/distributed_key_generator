use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::generate_partial_key;
use tracing::info;

use crate::{
    rpc::{cluster::request_submit_partial_key::submit_partial_key_to_leader, prelude::*},
    utils::{time::get_current_timestamp, log::log_prefix_role_and_address},
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

// TODO (Post-PoC): Decouple session start trigger from decryption key sync to improve robustness.
impl RpcParameter<AppState> for SyncDecryptionKey {
    type Response = ();

    fn method() -> &'static str {
        "sync_decryption_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
        let mut session_id = self.payload.session_id;
        // let sender_address = verify_signature(&self.signature, &self.payload, &_context)?;

        // TODO: Before storing the decryption key,
        // - Verify the signature on the decryption key payload
        // - Retrieve the previously stored encryption key for the session
        // - Verify that the decryption key is correctly derived from the encryption key
        // Only after successful verification, store the decryption key with put.
        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());
        decryption_key.put(session_id)?;

        info!(
            "{} Completed putting aggregated key - current_session_id: {:?}",
            prefix,
            self.payload.session_id.as_u64(),
        );

        // TODO: Change it to Random Delay? for not deterministic behavior
        // sleep(Duration::from_millis(1000)).await;

        let skde_params = context.skde_params();
        let (_, partial_key) = generate_partial_key(skde_params).unwrap();
        session_id.increase_session_id();

        // session_id is increased by 1
        submit_partial_key_to_leader(session_id, partial_key, &context.clone()).await?;

        info!(
            "{} Completed submitting partial key - session_id: {:?}",
            prefix,
            self.payload.session_id.as_u64()
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
    let prefix = log_prefix_role_and_address(&context.config());
    let ack_solve_timestamp = get_current_timestamp();
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    info!(
        "{} Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}",
        prefix, session_id, all_key_generator_rpc_url_list
    );

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
