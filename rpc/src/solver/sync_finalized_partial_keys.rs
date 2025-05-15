use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;
use tracing::{error, info, warn};

use super::submit_decryption_key::{
    DecryptionKeyResponse, SubmitDecryptionKey, SubmitDecryptionKeyPayload,
};
use crate::{
    get_current_timestamp,
    rpc::{
        common::{
            process_partial_key_submissions, validate_partial_key_submission,
            SyncFinalizedPartialKeysPayload,
        },
        prelude::*,
    },
    utils::{
        key::{
            calculate_decryption_key, perform_randomized_aggregation,
            verify_encryption_decryption_key_pair,
        },
        log::log_prefix_role_and_address,
        signature::create_signature,
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
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(context.config());

        PartialKeyAddressList::initialize(self.payload.session_id)?;

        validate_partial_key_submission(&self.signature, &self.payload)?;

        let partial_keys = process_partial_key_submissions(&prefix, &self.payload)?;

        tokio::spawn(async move {
            if let Err(err) =
                derive_and_submit_decryption_key(&context, self.payload.session_id, &partial_keys)
                    .await
            {
                error!(
                    "{} Solve failed for session {}: {:?}",
                    prefix,
                    self.payload.session_id.as_u64(),
                    err
                );
            } else {
                info!(
                    "{} Solve completed successfully for session {}",
                    prefix,
                    self.payload.session_id.as_u64()
                );
            }
        });
        Ok(())
    }
}

async fn derive_and_submit_decryption_key(
    context: &AppState,
    session_id: SessionId,
    partial_keys: &[PartialKey],
) -> Result<(), RpcError> {
    let prefix = log_prefix_role_and_address(context.config());

    let aggregated_key = perform_randomized_aggregation(context, session_id, &partial_keys);

    let decryption_key = calculate_decryption_key(context, session_id, &aggregated_key)
        .unwrap()
        .as_string();

    let encryption_key = aggregated_key.u;

    verify_encryption_decryption_key_pair(
        context.skde_params(),
        &encryption_key,
        decryption_key.as_str(),
        &prefix,
    )?;

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
