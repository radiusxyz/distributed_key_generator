use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::{RpcError, RpcParameter},
    },
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{
    verify_partial_key_validity, PartialKey as SkdePartialKey, PartialKeyProof,
};
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{
        common::{create_signature, get_current_timestamp},
        prelude::*,
    },
};

// TODO: Change structure name to SyncPartialKey, SyncPartialKeyPayload
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitPartialKeyAck {
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

impl RpcParameter<AppState> for SubmitPartialKeyAck {
    type Response = ();

    fn method() -> &'static str {
        "ack_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        info!(
            "Received partial key ACK - sender:{:?}, receipent:{:?}, session_id: {:?
            }, index: {}, timestamp: {}",
            self.payload.partial_key_sender,
            context.config().address(),
            self.payload.session_id,
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

        let is_valid = verify_partial_key_validity(
            context.skde_params(),
            self.payload.partial_key.clone(),
            self.payload.proof.clone(),
        )
        .unwrap();

        if !is_valid {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                format!(
                    "sender: {:?}, partial_key: {:?}",
                    self.payload.partial_key_sender, self.payload.partial_key
                ),
            )));
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
    proof: PartialKeyProof,
    submit_timestamp: u64,
    index: usize,
    _context: &AppState,
) -> Result<(), Error> {
    let all_key_generator_rpc_url_list =
        KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();

    info!(
        "Broadcasting partial key acknowledgment - session_id: {:?}, index: {}, timestamp: {}",
        session_id, index, submit_timestamp
    );

    let ack_timestamp = get_current_timestamp();

    let payload = PartialKeyAckPayload {
        partial_key_sender: sender_address,
        session_id,
        partial_key,
        proof,
        index,
        submit_timestamp,
        ack_timestamp,
    };

    // TODO: Add to make actual signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

    let parameter = SubmitPartialKeyAck { signature, payload };

    tokio::spawn(async move {
        if let Ok(rpc_client) = RpcClient::new() {
            let _ = rpc_client
                .multicast(
                    all_key_generator_rpc_url_list,
                    SubmitPartialKeyAck::method(),
                    &parameter,
                    Id::Null,
                )
                .await;
        }
    });

    Ok(())
}
