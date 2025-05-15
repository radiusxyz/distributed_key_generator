
use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;
use radius_sdk::signature::Signature;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSkdeParams {
    pub params: SkdeParams,
    pub signature: Signature,
}