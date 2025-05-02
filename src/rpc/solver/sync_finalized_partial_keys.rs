use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::submit_decryption_key::{
    DecryptionKeyResponse, SubmitDecryptionKey, SubmitDecryptionKeyPayload,
};
use crate::{
    error::KeyGenerationError,
    get_current_timestamp,
    rpc::{common::SyncFinalizedPartialKeysPayload, prelude::*},
    utils::{
        key::{calculate_decryption_key, perform_randomized_aggregation},
        log::log_prefix_role_and_address,
        signature::{create_signature, verify_signature},
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SolverSyncFinalizedPartialKeys {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

impl RpcParameter<AppState> for SolverSyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let SyncFinalizedPartialKeysPayload {
            sender,
            partial_key_submissions,
            session_id,
            ..
        } = &self.payload;
        let sender_address = verify_signature(&self.signature, &self.payload)?;
        if sender_address != sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let prefix = log_prefix_role_and_address(context.config());

        let payload = self.payload.clone();

        info!(
            "{} Received finalized partial keys ACK - partial_key_submissions.len(): {:?}, session_id: {:?
            }, timestamp: {}",
            prefix,
            payload.partial_key_submissions.len(),
            session_id,
            payload.ack_timestamp
        );

        PartialKeyAddressList::initialize(*session_id)?;

        for (i, pk_submission) in partial_key_submissions.iter().enumerate() {
            let sender = pk_submission.payload.sender.clone();
            let signable_message = pk_submission.payload.clone();

            // TODO: Signature verification
            let signer = verify_signature(&pk_submission.signature, &signable_message)?;
            if signer != sender {
                return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                    format!(
                        "[Cluster] Signature mismatch at index {}: expected {:?}, got {:?}",
                        i, sender, signer
                    ),
                )));
            }
            PartialKeyAddressList::apply(self.payload.session_id, |list| {
                list.insert(sender.clone());
            })?;

            PartialKeySubmission::new(pk_submission).put(self.payload.session_id, &sender)?;
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
    let partial_key_submissions =
        PartialKeyAddressList::get(session_id)?.get_partial_key_list(session_id)?;

    let partial_keys: Vec<_> = partial_key_submissions
        .iter()
        .map(|partial_key_submission| partial_key_submission.payload.partial_key.clone())
        .collect();

    // Put aggregated key for a Solver
    let aggregated_key = perform_randomized_aggregation(context, session_id, &partial_keys);

    let decryption_key = calculate_decryption_key(context, session_id, &aggregated_key)
        .unwrap()
        .as_string();

    // let decryption_key = decrypted.sk.clone();
    DecryptionKey::new(decryption_key.clone()).put(session_id)?;

    // Submit to leader
    let node = context.config().signer();
    let leader_rpc_url = context.config().leader_solver_rpc_url().clone().unwrap();

    let payload = SubmitDecryptionKeyPayload {
        sender: node.address().clone(),
        decryption_key: decryption_key.clone(),
        session_id,
        timestamp: get_current_timestamp(),
    };

    let timestamp = payload.timestamp;
    let signature = create_signature(node, &payload).unwrap();
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
