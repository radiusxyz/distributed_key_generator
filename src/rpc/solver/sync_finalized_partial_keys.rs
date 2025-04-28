use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::{error, info, warn};

use super::submit_decryption_key::{
    DecryptionKeyResponse, SubmitDecryptionKey, SubmitDecryptionKeyPayload,
};
use crate::{
    error::KeyGenerationError,
    get_current_timestamp,
    rpc::prelude::*,
    utils::{
        key::{calculate_decryption_key, perform_randomized_aggregation},
        log::log_prefix_role_and_address,
        signature::{create_signature, verify_signature},
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
pub struct SyncFinalizedPartialKeys {
    pub sender: Address,
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub partial_key_senders: Vec<Address>,
    pub partial_keys: Vec<SkdePartialKey>,
    pub session_id: SessionId,
    pub submit_timestamps: Vec<u64>,
    pub signatures: Vec<Signature>,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let sender_address = verify_signature(&self.signature, &self.payload)?;
        if &sender_address != &self.sender {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_role_and_address(&context.config());

        let payload = self.payload.clone();

        info!(
            "{} Received finalized partial keys ACK - senders:{:?}, session_id: {:?
            }, timestamp: {}",
            prefix, payload.partial_key_senders, payload.session_id, payload.ack_timestamp
        );

        if payload.partial_key_senders.len() != payload.partial_keys.len()
            || payload.partial_keys.len() != payload.submit_timestamps.len()
        {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                "Mismatched vector lengths in partial key ACK payload".into(),
            )));
        }

        for (sig, sender) in payload.signatures.iter().zip(payload.partial_key_senders) {
            let signer = verify_signature(sig, &self.payload)?;

            if &signer != &sender {
                return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                    "Signature does not match sender".into(),
                )));
            }
        }
        PartialKeyAddressList::initialize(payload.session_id)?;

        for (_i, (((sender, key), timestamp), sig)) in self
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

            let _signer = verify_signature(sig, &signable_message)?;
            // if &signer != sender {
            //     return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
            //         format!(
            //             "Signature mismatch at index {:?}: expected {:?}, got {:?}",
            //             i, sender, signer
            //         ),
            //     )));
            // }
            PartialKeyAddressList::apply(payload.session_id, |list| {
                list.insert(sender.clone());
            })?;
            PartialKey::new(key.clone()).put(payload.session_id, &sender)?;
        }

        tokio::spawn(async move {
            if let Err(err) = derive_and_submit_decryption_key(&context, payload.session_id).await {
                error!(
                    "{} Solve failed for session {}: {:?}",
                    prefix,
                    payload.session_id.as_u64(),
                    err
                );
            } else {
                info!(
                    "{} Solve completed successfully for session {}",
                    prefix,
                    payload.session_id.as_u64()
                );
            }
        });
        Ok(())
    }
}

async fn derive_and_submit_decryption_key(
    context: &AppState,
    session_id: SessionId,
) -> Result<(), Error> {
    let prefix = log_prefix_role_and_address(&context.config());
    let partial_keys = PartialKeyAddressList::get(session_id)?.get_partial_key_list(session_id)?;

    // Put aggregated key for a Solver
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
    let signature = create_signature(context, &bincode::serialize(&payload).unwrap()).unwrap();
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
            "{} Successfully submitted decryption key : session_id: {:?
            }, timestamp: {}",
            prefix, session_id, timestamp
        );
    } else {
        warn!(
            "{} Submission acknowledged but not successful : session_id: {:?
            }, timestamp: {}",
            prefix, session_id, timestamp
        );
    }

    Ok(())
}
