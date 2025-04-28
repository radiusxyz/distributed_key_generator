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
    get_current_timestamp,
    rpc::prelude::*,
    utils::{
        calculate_decryption_key, create_signature, log_prefix_role_and_address,
        perform_randomized_aggregation,
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
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub partial_key_submissions: Vec<PartialKeySubmission>,
    pub session_id: SessionId,
    pub ack_timestamp: u64,
}

impl RpcParameter<AppState> for SyncFinalizedPartialKeys {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
        // let sender_address = verify_signature(&self.signature, &self.payload)?;

        let payload = self.payload.clone();

        info!(
            "{} Received finalized partial keys ACK - partial_key_submissions.len(): {:?}, session_id: {:?
            }, timestamp: {}",
            prefix,
            payload.partial_key_submissions.len(),
            payload.session_id,
            payload.ack_timestamp
        );

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

        for (_, partial_key_submission) in payload.partial_key_submissions.iter().enumerate() {
            let sender = partial_key_submission.payload.sender.clone();
            let _signable_message = PartialKeyPayload {
                sender: sender.clone(),
                partial_key: partial_key_submission.payload.partial_key.clone(),
                submit_timestamp: partial_key_submission.payload.submit_timestamp,
                session_id: payload.session_id,
            };

            // TODO: Signature verification
            // if &signer != sender {
            //     return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
            //         format!(
            //             "Signature mismatch at index {:?}: expected {:?}, got {:?}",
            //             i, sender, signer
            //         ),
            //     )));
            // }
            PartialKeyAddressList::apply(payload.session_id, |list| {
                list.insert(partial_key_submission.payload.sender.clone());
            })?;
            PartialKeySubmission::clone_from(partial_key_submission)
                .put(payload.session_id, &sender)?;
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

    let partial_keys: Vec<SkdePartialKey> = partial_key_submissions
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
