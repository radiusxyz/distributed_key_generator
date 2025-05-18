use crate::primitives::*;
use dkg_primitives::{SignedSkdeParams, AppState};
use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParamsResponse<Signature> {
    pub signed_skde_params: SignedSkdeParams<Signature>,
}

impl<C> RpcParameter<C> for GetAuthorizedSkdeParams
where
    C: AppState,
{
    type Response = GetAuthorizedSkdeParamsResponse<C::Signature>;

    fn method() -> &'static str {
        "get_authorized_skde_params"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = context
            .sign(&skde_params)
            .map_err(|e| RpcError::from(e))?;
        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };
        Ok(GetAuthorizedSkdeParamsResponse { signed_skde_params })
    }
}

impl<Signature> From<SignedSkdeParams<Signature>> for GetAuthorizedSkdeParamsResponse<Signature> {
    fn from(value: SignedSkdeParams<Signature>) -> Self {
        Self { signed_skde_params: value }
    }
}

impl<Signature> Into<SkdeParams> for GetAuthorizedSkdeParamsResponse<Signature> {
    fn into(self) -> SkdeParams {
        self.signed_skde_params.params
    }
}
