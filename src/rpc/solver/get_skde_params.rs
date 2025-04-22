use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse {
    skde_params: skde::delay_encryption::SkdeParams,
}

impl GetSkdeParamsResponse {
    pub fn into_skde_params(self) -> skde::delay_encryption::SkdeParams {
        self.skde_params
    }
}

impl RpcParameter<AppState> for GetSkdeParams {
    type Response = GetSkdeParamsResponse;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();

        Ok(GetSkdeParamsResponse {
            skde_params: skde_params.clone(),
        })
    }
}
