use dkg_types::SignedSkdeParams;
use skde::delay_encryption::SkdeParams;

use crate::primitives::*;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GetSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl GetSkdeParamsResponse {
    pub fn into_skde_params(self) -> SkdeParams {
        self.signed_skde_params.params
    }
}

impl RpcParameter<AppState> for GetSkdeParams {
    type Response = GetSkdeParamsResponse;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = create_signature(context.config().signer(), &skde_params).unwrap();
        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };

        Ok(GetSkdeParamsResponse { signed_skde_params })
    }
}
