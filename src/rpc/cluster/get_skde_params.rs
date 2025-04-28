use crate::{rpc::prelude::*, task::authority_setup::SignedSkdeParams, utils::signature::create_signature};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl GetSkdeParamsResponse {
    pub fn into_skde_params(self) -> skde::delay_encryption::SkdeParams {
        self.signed_skde_params.params
    }
}

impl RpcParameter<AppState> for GetSkdeParams {
    type Response = GetSkdeParamsResponse;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params().clone();

        let signature = create_signature(&context, &skde_params).unwrap();

        let signed_skde_params = SignedSkdeParams {
            params: skde_params,
            signature,
        };

        Ok(GetSkdeParamsResponse { signed_skde_params })
    }
}
