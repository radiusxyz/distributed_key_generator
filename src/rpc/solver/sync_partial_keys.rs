use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::{info, warn};

use super::submit_decryption_key::{
    DecryptionKeyResponse, SubmitDecryptionKey, SubmitDecryptionKeyPayload,
};
use crate::{
    error::KeyGenerationError,
    get_current_timestamp,
    rpc::prelude::*,
    utils::{
        calculate_decryption_key, create_signature, perform_randomized_aggregation,
        verify_signature, AddressExt,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload {
    pub sender: Address,
    pub partial_key: SkdePartialKey,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeys {
    pub signature: Signature,
    pub payload: SyncPartialKeysPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeysPayload {
    pub partial_key_senders: Vec<Address>,
    pub partial_keys: Vec<SkdePartialKey>,
    pub session_id: SessionId,
    pub submit_timestamps: Vec<u64>,
    pub signatures: Vec<Signature>,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        let payload = self.payload.clone();

        info!(
            "[{}] Received partial keys ACK - senders:{:?}, session_id: {:?
            }, timestamp: {}",
            context.config().address().to_short(),
            payload.partial_key_senders,
            payload.session_id,
            payload.ack_timestamp
        );

        if payload.partial_key_senders.len() != payload.partial_keys.len()
            || payload.partial_keys.len() != payload.submit_timestamps.len()
        {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                "Mismatched vector lengths in partial key ACK payload".into(),
            )));
        }
        // TODO: Signature verification
        // for (sig, sender) in signatures.iter().zip(partial_key_senders) {
        //     let signer = verify_signature(sig, &self.payload)?;

        //     if &signer != sender {
        //         return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
        //             "Signature does not match sender".into(),
        //         )));
        //     }
        // }
        PartialKeyAddressList::initialize(payload.session_id)?;

        for (i, (((sender, key), timestamp), sig)) in self
            .payload
            .partial_key_senders
            .iter()
            .zip(&self.payload.partial_keys)
            .zip(&self.payload.submit_timestamps)
            .zip(&self.payload.signatures)
            .enumerate()
        {
            let signable_message = PartialKeyPayload {
                sender: sender.clone(),
                partial_key: key.clone(),
                submit_timestamp: *timestamp,
                session_id: payload.session_id,
            };

            let signer = verify_signature(sig, &signable_message)?;

            if &signer != sender {
                return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                    format!(
                        "Signature mismatch at index {:?}: expected {:?}, got {:?}",
                        i, sender, signer
                    ),
                )));
            }
            PartialKeyAddressList::apply(payload.session_id, |list| {
                list.insert(sender.clone());
            })?;
            PartialKey::new(key.clone()).put(payload.session_id, &sender)?;
        }

        tokio::spawn(async move {
            if let Err(err) = process_key_derivation(&context, payload.session_id).await {
                tracing::error!(
                    "[{}] Solve failed for session {}: {:?}",
                    context.config().address().to_short(),
                    payload.session_id.as_u64(),
                    err
                );
            } else {
                tracing::info!(
                    "[{}] Solve completed successfully for session {}",
                    context.config().address().to_short(),
                    payload.session_id.as_u64()
                );
            }
        });
        Ok(())
    }
}

async fn process_key_derivation(context: &AppState, session_id: SessionId) -> Result<(), Error> {
    let partial_keys = PartialKeyAddressList::get(session_id)?.get_partial_key_list(session_id)?;

    let aggregated_key = perform_randomized_aggregation(context, session_id, &partial_keys);

    let decryption_key = calculate_decryption_key(context, session_id, &aggregated_key)
        .unwrap()
        .as_string();

    // let decryption_key = decrypted.sk.clone();
    DecryptionKey::new(decryption_key.clone()).put(session_id)?;

    // Submit to leader
    let sender = context.config().signer().address();
    let leader_rpc_url = context.config().leader_solver_rpc_url().clone().unwrap();

    let payload = SubmitDecryptionKeyPayload {
        sender: sender.clone(),
        decryption_key: decryption_key.clone(),
        session_id,
        timestamp: get_current_timestamp(),
    };

    let timestamp = payload.timestamp;
    let signature = create_signature(&bincode::serialize(&payload).unwrap());
    let request = SubmitDecryptionKey { signature, payload };

    let rpc_client = RpcClient::new()?;
    let response: DecryptionKeyResponse = rpc_client
        .request(
            leader_rpc_url,
            SubmitDecryptionKey::method(),
            &request,
            Id::Null,
        )
        .await?;

    if response.success {
        info!(
            "[{}] Successfully submitted decryption key : session_id: {:?
            }, timestamp: {}",
            context.config().address().to_short(),
            session_id,
            timestamp
        );
    } else {
        warn!(
            "[{}] Submission acknowledged but not successful : session_id: {:?
            }, timestamp: {}",
            context.config().address().to_short(),
            session_id,
            timestamp
        );
    }

    Ok(())
}
