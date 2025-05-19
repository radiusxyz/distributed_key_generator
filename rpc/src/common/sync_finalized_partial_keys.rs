use crate::primitives::*;
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;
use dkg_primitives::{AppState, PartialKeyAddressList, SyncFinalizedPartialKeysPayload};

pub fn validate_partial_key_submission<C>(
    context: &C,
    signature: &C::Signature,
    payload: &SyncFinalizedPartialKeysPayload<C::Signature, C::Address>,
) -> Result<(), RpcError> 
where
    C: AppState,
{
    let _ = context.verify_signature(signature, payload, Some(&payload.sender))?;
    Ok(())
}

pub fn process_partial_key_submissions<C: AppState>(
    context: &C,
    payload: &SyncFinalizedPartialKeysPayload<C::Signature, C::Address>,
) -> Result<Vec<SkdePartialKey>, RpcError> {
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
