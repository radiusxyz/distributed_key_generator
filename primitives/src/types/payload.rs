
use radius_sdk::signature::Address;
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;

use crate::{PartialKeySubmission, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub sender: Address,
    pub partial_key_submissions: Vec<PartialKeySubmission>,
    pub session_id: SessionId,
    pub ack_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload {
    pub sender: Address,
    pub partial_key: PartialKey,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}