
use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSkdeParams<Signature> {
    pub params: SkdeParams,
    pub signature: Signature,
}