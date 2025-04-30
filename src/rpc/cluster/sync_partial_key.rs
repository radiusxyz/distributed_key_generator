use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::prelude::*,
    utils::{
        log::{log_prefix_role_and_address, AddressExt},
        signature::{create_signature, verify_signature},
        time::get_current_timestamp,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey {
    pub signature: Signature,
    pub payload: SyncPartialKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeyPayload {
    pub sender: Address,
    pub partial_key_sender: Address,
    pub partial_key: SkdePartialKey,
    pub index: usize, // TODO: Remove this field
    pub session_id: SessionId,
    pub submit_timestamp: u64,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;
        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_role_and_address(context.config());

        info!(
            "{} Received partial key ACK - sender:{:?}, session_id: {:?
            }, index: {}, timestamp: {}",
            prefix,
            self.payload.partial_key_sender.to_short(),
            self.payload.session_id.as_u64(),
            self.payload.index,
            self.payload.ack_timestamp
        );

        // TODO: Leader verification (only leader can send ACK)

        // TODO: Store and process partial key index information
        // (In actual implementation, a structure to store index information is needed)
        // If sender is me, ignore
        if self.payload.partial_key_sender == context.config().address() {
            return Ok(());
        }

        PartialKeyAddressList::initialize(self.payload.session_id)?;

        // if the sender is incluided in
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(self.payload.partial_key_sender.clone());
        })?;

        let partial_key = PartialKey::new(self.payload.partial_key);
        partial_key.put(self.payload.session_id, &self.payload.partial_key_sender)?;

        Ok(())
    }
}

// Broadcast partial key acknowledgment from leader to the entire network
pub fn broadcast_partial_key_ack(
    sender_address: Address,
    session_id: SessionId,
    partial_key: SkdePartialKey,
    submit_timestamp: u64,
    index: usize,
    context: &AppState,
) -> Result<(), Error> {
    let prefix = log_prefix_role_and_address(context.config());
    let key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_other_key_generator_rpc_url_list(context.config().address());

    info!(
        "{} Broadcasting partial key acknowledgment - sender: {}, session_id: {:?}, timestamp: {}",
        prefix,
        sender_address.to_short(),
        session_id,
        submit_timestamp
    );

    let ack_timestamp = get_current_timestamp();

    let payload = SyncPartialKeyPayload {
        sender: context.config().address().clone(),
        partial_key_sender: sender_address,
        session_id,
        partial_key,
        index,
        submit_timestamp,
        ack_timestamp,
    };

    let signature = create_signature(
        context.config().signer(),
        &serialize_to_bincode(&payload).unwrap(),
    )
    .unwrap();

    let parameter = SyncPartialKey { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    key_generator_rpc_url_list,
                    SyncPartialKey::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
