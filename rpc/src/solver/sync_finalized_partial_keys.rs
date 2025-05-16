use crate::{
    primitives::*,
    common::process_partial_key_submissions,
    solver::submit_decryption_key::{
        DecryptionKeyResponse, SubmitDecryptionKey, SubmitDecryptionKeyPayload,
    }
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;
use tracing::{error, info, warn};
use dkg_primitives::{AppState, KeyGenerationError, SessionId, DecryptionKey, PartialKeyAddressList, SyncFinalizedPartialKeysPayload};
use dkg_utils::key::{
    calculate_decryption_key, perform_randomized_aggregation,
    verify_encryption_decryption_key_pair,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SolverSyncFinalizedPartialKeys<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload<Signature, Address>,
}

impl<C: AppState> RpcParameter<C> for SolverSyncFinalizedPartialKeys<C::Signature, C::Address>
where
    C: 'static,
{
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();

        PartialKeyAddressList::initialize(self.payload.session_id)?;
        let sender_address = context.verify_signature(&self.signature, &self.payload)?;
        if sender_address != self.payload.sender {
            return Err(RpcError::from(KeyGenerationError::InternalError(
                "Signature does not match sender address".into(),
            )));
        }

        let partial_keys = process_partial_key_submissions(&prefix, &self.payload)?;

        tokio::spawn(async move {
            if let Err(err) =
                derive_and_submit_decryption_key(&context, self.payload.session_id, &partial_keys)
                    .await
            {
                error!(
                    "{} Solve failed for session {:?}: {:?}",
                    prefix,
                    self.payload.session_id,
                    err
                );
            } else {
                info!(
                    "{} Solve completed successfully for session {:?}",
                    prefix,
                    self.payload.session_id
                );
            }
        });
        Ok(())
    }
}

async fn derive_and_submit_decryption_key<C: AppState>(
    context: &C,
    session_id: SessionId,
    partial_keys: &[PartialKey],
) -> Result<(), RpcError> {
    let prefix = context.log_prefix();

    let aggregated_key = perform_randomized_aggregation(context, session_id, &partial_keys);

    let decryption_key: String = calculate_decryption_key(context, session_id, &aggregated_key)
        .unwrap()
        .into();

    let encryption_key = aggregated_key.u;

    verify_encryption_decryption_key_pair(
        &context.skde_params(),
        &encryption_key,
        &decryption_key,
        &prefix,
    )?;

    DecryptionKey::new(decryption_key.clone()).put(session_id)?;

    let payload = SubmitDecryptionKeyPayload::new(
        context.address(),
        decryption_key.clone(),
        session_id,
    );

    let timestamp = payload.timestamp;
    let signature = context.create_signature(&payload)?;
    let request = SubmitDecryptionKey { signature, payload };

    let rpc_client = RpcClient::new()?;
    let response: DecryptionKeyResponse = rpc_client
        .request(
            context.leader_rpc_url(),
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
