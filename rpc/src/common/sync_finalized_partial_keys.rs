use crate::primitives::*;
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;
use dkg_primitives::{AppState, PartialKeyAddressList, KeyGenerationError, SyncFinalizedPartialKeysPayload};
use dkg_utils::signature::verify_signature;

pub fn validate_partial_key_submission<C>(
    context: &C,
    signature: &C::Signature,
    payload: &SyncFinalizedPartialKeysPayload,
) -> Result<(), RpcError> 
where
    C: AppState,
{
    let sender_address = context.verify_signature(signature, payload)?;
    if sender_address != payload.sender {
        return Err(RpcError::from(KeyGenerationError::InternalError(
            "Signature does not match sender address".into(),
        )));
    }

    Ok(())
}

pub fn process_partial_key_submissions(
    prefix: &str,
    payload: &SyncFinalizedPartialKeysPayload,
) -> Result<Vec<SkdePartialKey>, RpcError> {
    let SyncFinalizedPartialKeysPayload {
        partial_key_submissions,
        session_id,
        ack_timestamp,
        ..
    } = payload;

    info!(
        "{} Received finalized partial keys - partial_key_submissions.len(): {:?}, session_id: {:?}, timestamp: {}",
        prefix,
        partial_key_submissions.len(),
        session_id,
        ack_timestamp
    );

    let mut partial_keys = Vec::new();

    // TODO: Should use the proper index to order the partial keys
    let mut sorted_submissions = partial_key_submissions.clone();
    sorted_submissions.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    for (i, pk_submission) in sorted_submissions.iter().enumerate() {
        let signable_message = pk_submission.payload.clone();
        let signer = verify_signature(&pk_submission.signature, &signable_message)?;

        if signer != pk_submission.payload.sender {
            info!(
                "Signature mismatch at index {}: expected {:?}, got {:?}",
                i, pk_submission.payload.sender, signer
            );
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                format!(
                    "Signature mismatch at index {}: expected {:?}, got {:?}",
                    i, pk_submission.payload.sender, signer
                ),
            )));
        }

        PartialKeyAddressList::initialize(*session_id)?;
        PartialKeyAddressList::apply(*session_id, |list| {
            list.insert(signer.clone());
        })?;

        pk_submission.clone().put(*session_id, &signer)?;
        partial_keys.push(pk_submission.payload.partial_key.clone());
    }

    Ok(partial_keys)
}
