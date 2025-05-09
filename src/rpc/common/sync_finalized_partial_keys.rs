use radius_sdk::{json_rpc::server::RpcError, signature::Signature};
use skde::key_generation::PartialKey as SkdePartialKey;
use tracing::info;

use crate::{
    error::KeyGenerationError,
    rpc::{common::SyncFinalizedPartialKeysPayload, prelude::*},
    utils::signature::verify_signature,
};

pub fn validate_partial_key_submission(
    signature: &Signature,
    payload: &SyncFinalizedPartialKeysPayload,
) -> Result<(), RpcError> {
    let sender_address = verify_signature(signature, payload)?;
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
        "{} Received finalized partial keys ACK - partial_key_submissions.len(): {:?}, session_id: {:?}, timestamp: {}",
        prefix,
        partial_key_submissions.len(),
        session_id,
        ack_timestamp
    );

    let mut partial_keys = Vec::new();

    // TODO: Should fix it. Just test logic for fixed order of partial keys
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

        PartialKeySubmission::new(pk_submission).put(*session_id, &signer)?;
        partial_keys.push(pk_submission.payload.partial_key.clone());
    }

    Ok(partial_keys)
}
