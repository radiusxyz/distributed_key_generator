use crate::primitives::*;
use dkg_primitives::{SignedSkdeParams, AppState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse<Signature> {
    pub signed_skde_params: SignedSkdeParams<Signature>,
}

impl<C: AppState> RpcParameter<C> for GetSkdeParams {
    type Response = GetSkdeParamsResponse<C::Signature>;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = context.sign(&skde_params)?;
        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };

        Ok(GetSkdeParamsResponse { signed_skde_params })
    }
}
