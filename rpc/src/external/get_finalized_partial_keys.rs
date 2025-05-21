use crate::primitives::*;
use dkg_primitives::{
    AppState, SessionId, PartialKeySubmission, SubmitterList, KeyGenerationError
};
use serde::{Deserialize, Serialize};

/// Get finalized partial keys for a specified session ID
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedPartialKeys {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedPartialKeysResponse<Signature, Address> {
    pub partial_keys: Vec<PartialKeySubmission<Signature, Address>>,
}

impl<C: AppState> RpcParameter<C> for GetFinalizedPartialKeys {
    type Response = GetFinalizedPartialKeysResponse<C::Signature, C::Address>;

    fn method() -> &'static str {
        "get_finalized_partial_keys"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        let session_id = self.session_id;
        let partial_keys = SubmitterList::<C::Address>::get(session_id)?
            .get_partial_keys::<C>(session_id)
            .map_err(|err| {
                RpcError::from(KeyGenerationError::InternalError(
                    format!("Failed to get partial key list: {:?}", err).into(),
                ))
            })?;
        Ok(GetFinalizedPartialKeysResponse { partial_keys })
    }
}
