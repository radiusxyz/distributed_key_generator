use crate::primitives::*;
use dkg_primitives::{SignedSkdeParams, AppState};
use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl<C: AppState> RpcParameter<C> for GetAuthorizedSkdeParams {
    type Response = GetAuthorizedSkdeParamsResponse;

    fn method() -> &'static str {
        "get_authorized_skde_params"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = context.create_signature(&skde_params).unwrap();
        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };
        Ok(GetAuthorizedSkdeParamsResponse { signed_skde_params })
    }
}

impl From<SignedSkdeParams> for GetAuthorizedSkdeParamsResponse {
    fn from(signed: SignedSkdeParams) -> Self {
        Self {
            signed_skde_params: signed,
        }
    }
}

impl GetAuthorizedSkdeParamsResponse {
    pub fn into_skde_params(self) -> SkdeParams {
        self.signed_skde_params.params
    }
}
