use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParamsResponse {
    skde_params: skde::delay_encryption::SkdeParams,
}

impl GetSkdeParams {
    pub const METHOD_NAME: &'static str = "get_skde_params";

    pub async fn handler(
        _parameter: RpcParameter,
        context: Arc<AppState>,
    ) -> Result<GetSkdeParamsResponse, RpcError> {
        let skde_params = context.skde_params();

        return Ok(GetSkdeParamsResponse {
            skde_params: skde_params.clone(),
        });
    }
}
