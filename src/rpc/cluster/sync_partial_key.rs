use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    rpc::prelude::*,
    utils::{create_signature, get_current_timestamp, log_prefix_role_and_address, AddressExt},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey {
    pub signature: Signature,
    pub payload: SyncPartialKeyPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeyPayload {
    pub partial_key_submission: PartialKeySubmission,
    pub session_id: SessionId,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "{} Received partial key ACK - sender:{:?}, session_id: {:?
            }, timestamp: {}",
            prefix,
            self.payload
                .partial_key_submission
                .payload
                .sender
                .to_short(),
            self.payload.session_id.as_u64(),
            self.payload.ack_timestamp
        );

        // TODO: Leader verification (only leader can send ACK)

        // If sender is me, ignore
        if self.payload.partial_key_submission.payload.sender == context.config().address() {
            return Ok(());
        }

        PartialKeyAddressList::initialize(self.payload.session_id)?;

        // if the sender is incluided in
        PartialKeyAddressList::apply(self.payload.session_id, |list| {
            list.insert(self.payload.partial_key_submission.payload.sender.clone());
        })?;

        let partial_key_submission =
            PartialKeySubmission::clone_from(&self.payload.partial_key_submission);
        partial_key_submission.put(
            self.payload.session_id,
            &self.payload.partial_key_submission.payload.sender,
        )?;

        Ok(())
    }
}

// Broadcast partial key acknowledgment from leader to the entire network
pub fn broadcast_partial_key_ack(
    sender_address: Address,
    partial_key_submission: PartialKeySubmission,
    context: &AppState,
) -> Result<(), Error> {
    let prefix = log_prefix_role_and_address(&context.config());
    let key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_other_key_generator_rpc_url_list(&context.config().address());

    info!(
        "{} Broadcasting partial key acknowledgment - sender: {}, session_id: {:?}, timestamp: {}",
        prefix,
        sender_address.to_short(),
        partial_key_submission.payload.session_id,
        partial_key_submission.payload.submit_timestamp
    );

    let ack_timestamp = get_current_timestamp();

    let payload = SyncPartialKeyPayload {
        partial_key_submission: partial_key_submission.clone(),
        session_id: partial_key_submission.payload.session_id,
        ack_timestamp,
    };

    // TODO: Add to make actual signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

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
