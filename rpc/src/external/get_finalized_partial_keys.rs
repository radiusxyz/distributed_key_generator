use crate::primitives::*;
use dkg_primitives::{
    AppState, SessionId, PartialKeySubmission, SubmitterList, KeyGenerationError
};
use serde::{Deserialize, Serialize};

/// Get finalized partial keys at session id
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedPartialKeys {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response<Signature, Address> {
    pub partial_keys: Vec<PartialKeySubmission<Signature, Address>>,
}

impl<C: AppState> RpcParameter<C> for GetFinalizedPartialKeys {
    type Response = Response<C::Signature, C::Address>;

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
        Ok(Response { partial_keys })
    }
}
