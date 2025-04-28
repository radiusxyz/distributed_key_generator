use radius_sdk::signature::{Address, Signature};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey as SkdePartialKey;

use crate::SessionId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload {
    pub sender: Address,
    pub partial_key_senders: Vec<Address>,
    pub partial_keys: Vec<SkdePartialKey>,
    pub session_id: SessionId,
    pub submit_timestamps: Vec<u64>,
    pub signatures: Vec<Signature>,
    pub ack_timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload {
    pub sender: Address,
    pub partial_key: SkdePartialKey,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}
