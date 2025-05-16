use crate::primitives::*;
use dkg_primitives::{AppState, SessionId, Error, KeyGeneratorList};
use serde::{Deserialize, Serialize};
use skde::key_generation::generate_partial_key;
use tracing::{error, info};

use crate::{
    error::KeyGenerationError,
    rpc::{cluster::request_submit_partial_key::submit_partial_key_to_leader, prelude::*},
    utils::{
        key::verify_encryption_decryption_key_pair, log::log_prefix,
        signature::verify_signature, time::get_current_timestamp,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKey<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncDecryptionKeyPayload<Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKeyPayload<Address> {
    pub sender: Address,
    pub decryption_key: String,
    pub session_id: SessionId,
    pub solve_timestamp: u128,
    pub ack_solve_timestamp: u128,
}

// TODO (Post-PoC): Decouple session start trigger from decryption key sync to improve robustness.
impl<C: AppState> RpcParameter<C> for SyncDecryptionKey<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_decryption_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix(context.config());

        let sender_address = match verify_signature(&self.signature, &self.payload) {
            Ok(address) => address,
            Err(err) => {
                error!("{} Signature verification failed: {}", prefix, err);
                return Err(RpcError::from(err));
            }
        };

        if sender_address != self.payload.sender {
            let err_msg = "Signature does not match sender address";
            error!("{} {}", prefix, err_msg);
            return Err(RpcError::from(KeyGenerationError::InternalError(
                err_msg.into(),
            )));
        }

        let mut session_id = self.payload.session_id;
        let skde_params = context.skde_params();
        let encryption_key = AggregatedKey::get(session_id)?.encryption_key();
        let decryption_key = DecryptionKey::new(self.payload.decryption_key.clone());

        context.verify_decryption_key(
            &skde_params,
            &encryption_key,
            decryption_key.clone().as_string().as_str(),
            &prefix,
        )?;

        decryption_key.put(session_id)?;

        let (_, partial_key) = generate_partial_key(&skde_params).unwrap();
        session_id.increase_session_id();

        // session_id is increased by 1
        submit_partial_key_to_leader(session_id, partial_key, &context.clone()).await?;

        info!("{} Completed submitting partial key", prefix,);

        Ok(())
    }
}

// Broadcast decryption key acknowledgment from leader to the network
pub fn broadcast_decryption_key_ack(
    session_id: SessionId,
    decryption_key: String,
    solve_timestamp: u128,
    context: &AppState,
) -> Result<(), Error> {
    let prefix = log_prefix(context.config());
    let ack_solve_timestamp = get_current_timestamp();
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    info!(
        "{} Broadcast decryption key - session_id: {:?}, all_dkg_list: {:?}",
        prefix, session_id, all_key_generator_rpc_url_list
    );

    let payload = SyncDecryptionKeyPayload {
        sender: context.config().address().clone(),
        session_id,
        decryption_key,
        solve_timestamp,
        ack_solve_timestamp,
    };

    let signature = context.config().signer().sign_message(&payload).unwrap();

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
