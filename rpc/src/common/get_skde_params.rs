use crate::primitives::*;
use dkg_primitives::{SignedSkdeParams, AppState};
use skde::delay_encryption::SkdeParams;
use serde::{Deserialize, Serialize};

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

impl<C: AppState> RpcParameter<C> for GetSkdeParams {
    type Response = GetSkdeParamsResponse;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = context.create_signature(&skde_params)?;
        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };

        Ok(GetSkdeParamsResponse { signed_skde_params })
    }
}
