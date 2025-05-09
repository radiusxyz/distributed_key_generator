use crate::{error::KeyGenerationError, rpc::prelude::*};

/// Get finalized partial keys for a specified session ID
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedPartialKeys {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetFinalizedPartialKeysResponse {
    pub partial_key_submissions: Vec<PartialKeySubmission>,
}

impl RpcParameter<AppState> for GetFinalizedPartialKeys {
    type Response = GetFinalizedPartialKeysResponse;

    fn method() -> &'static str {
        "get_finalized_partial_keys"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let session_id = self.session_id;

        let partial_key_address_list = PartialKeyAddressList::get(session_id)?;

        let partial_key_submissions = partial_key_address_list
            .get_partial_key_list(session_id)
            .map_err(|err| {
                RpcError::from(KeyGenerationError::InternalError(
                    format!("Failed to get partial key list: {:?}", err).into(),
                ))
            })?;

        Ok(GetFinalizedPartialKeysResponse {
            partial_key_submissions,
        })
    }
}
