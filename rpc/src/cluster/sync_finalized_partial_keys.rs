use dkg_primitives::{
    AppState, DecryptionKey, Error, PartialKeyAddressList, SessionId, SubmitDecryptionKeyPayload, SyncFinalizedPartialKeysPayload
};
use dkg_utils::key::{
    calculate_decryption_key, perform_randomized_aggregation, verify_encryption_decryption_key_pair,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;
use tracing::{error, info, warn};

use crate::{
    primitives::*,
    cluster::{
        DecryptionKeyResponse, SubmitDecryptionKey,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeys<Signature, Address> {
    pub signature: Signature,
    pub payload: SyncFinalizedPartialKeysPayload<Signature, Address>,
}

impl<C: AppState> RpcParameter<C> for SyncFinalizedPartialKeys<C::Signature, C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_finalized_partial_keys"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        if context.is_solver() {
            PartialKeyAddressList::<C::Address>::initialize(self.payload.session_id)?;
            let _ = context.verify_signature(&self.signature, &self.payload, Some(&self.payload.sender))?;
            let partial_keys = process_partial_key_submissions::<C>(&context, &self.payload)?;
            let cloned_context = context.clone();
            cloned_context.spawn_task(Box::pin(
                async move {
                    if let Err(err) =
                        derive_and_submit_decryption_key::<C>(context, self.payload.session_id, &partial_keys)
                            .await
                    {
                        error!(
                            "Solve failed for session {:?}: {:?}",
                            self.payload.session_id, err
                        );
                    } else {
                        info!(
                            "Solve completed successfully for session {:?}",
                            self.payload.session_id
                        );
                    }
                }
            ));
        } else {
            let partial_keys: Vec<skde::key_generation::PartialKey> = process_partial_key_submissions::<C>(&context, &self.payload)?;
            perform_randomized_aggregation(&context, self.payload.session_id, &partial_keys);
        }
        Ok(())
    }
}

async fn derive_and_submit_decryption_key<C: AppState>(
    context: C,
    session_id: SessionId,
    partial_keys: &[PartialKey],
) -> Result<(), RpcError> {
    let prefix = context.log_prefix();

    let aggregated_key = perform_randomized_aggregation(&context, session_id, &partial_keys);

    let decryption_key: String = calculate_decryption_key(&context, session_id, &aggregated_key)
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

    let payload =
        SubmitDecryptionKeyPayload::new(context.address(), decryption_key.clone(), session_id);

    let timestamp = payload.timestamp;
    let signature = context.sign(&payload)?;
    let request = SubmitDecryptionKey { signature, payload };

    let rpc_client = RpcClient::new()?;
    let leader_rpc_url = context.leader_rpc_url().ok_or(Error::InvalidParams("Leader RPC URL is not set".to_string()))?;
    let response: DecryptionKeyResponse = rpc_client
        .request(
            &leader_rpc_url,
            <SubmitDecryptionKey::<C::Signature, C::Address> as RpcParameter<C>>::method(),
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

pub fn process_partial_key_submissions<C: AppState>(
    context: &C,
    payload: &SyncFinalizedPartialKeysPayload<C::Signature, C::Address>,
) -> Result<Vec<PartialKey>, RpcError> {
    let SyncFinalizedPartialKeysPayload {
        partial_key_submissions,
        session_id,
        ack_timestamp,
        ..
    } = payload;

    info!(
        "{} Received finalized partial keys - partial_key_submissions.len(): {:?}, session_id: {:?}, timestamp: {}",
        context.log_prefix(),
        partial_key_submissions.len(),
        session_id,
        ack_timestamp
    );

    let mut partial_keys = Vec::new();

    // TODO: Should use the proper index to order the partial keys
    let mut sorted_submissions = partial_key_submissions.clone();
    sorted_submissions.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    for pk_submission in sorted_submissions.iter() {
        let signable_message = pk_submission.payload.clone();
        let signer = context.verify_signature(&pk_submission.signature, &signable_message, Some(&pk_submission.sender()))?;
        PartialKeyAddressList::<C::Address>::initialize(*session_id)?;
        PartialKeyAddressList::apply(*session_id, |list| {
            list.insert(signer.clone());
        })?;
        pk_submission.clone().put(*session_id, signer)?;
        partial_keys.push(pk_submission.payload.partial_key.clone());
    }

    Ok(partial_keys)
}
