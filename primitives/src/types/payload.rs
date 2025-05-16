use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;

use crate::{PartialKeySubmission, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload<Signature, Address> {
    pub sender: Address,
    pub partial_key_submissions: Vec<PartialKeySubmission<Signature, Address>>,
    pub session_id: SessionId,
    pub ack_timestamp: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload<Address> {
    pub sender: Address,
    pub partial_key: PartialKey,
    pub submit_timestamp: u128,
    pub session_id: SessionId,
}

impl<Address> PartialKeyPayload<Address> {
    pub fn new(sender: Address, partial_key: PartialKey, session_id: SessionId) -> Self {
        let submit_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, partial_key, submit_timestamp, session_id }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeyPayload<Signature, Address> {
    pub sender: Address,
    pub partial_key_submission: PartialKeySubmission<Signature, Address>,
    pub session_id: SessionId,
    pub ack_timestamp: u128,
}

impl<Signature, Address> SyncPartialKeyPayload<Signature, Address> {
    pub fn new(sender: Address, partial_key_submission: PartialKeySubmission<Signature, Address>, session_id: SessionId) -> Self {
        let ack_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, partial_key_submission, session_id, ack_timestamp }
    }
}
