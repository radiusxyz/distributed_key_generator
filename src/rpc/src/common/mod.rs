mod get_skde_params;

use radius_sdk::signature::Address;
use serde::{Deserialize, Serialize};
use skde::{delay_encryption::SkdeParams, key_generation::PartialKey as SkdePartialKey};

use crate::{task::authority_setup::SignedSkdeParams, PartialKeySubmission, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl GetSkdeParamsResponse {
    pub fn into_skde_params(self) -> SkdeParams {
        self.signed_skde_params.params
    }
}

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
    pub partial_key: SkdePartialKey,
    pub submit_timestamp: u64,
    pub session_id: SessionId,
}
