use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParamsResponse {
    pub skde_params: SkdeParams,
}

impl RpcParameter<AppState> for GetAuthorizedSkdeParams {
    type Response = GetAuthorizedSkdeParamsResponse;

    fn method() -> &'static str {
        "get_authorized_skde_params"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        Ok(GetAuthorizedSkdeParamsResponse {
            skde_params: context.skde_params().clone(),
        })
    }
}

impl From<SkdeParams> for GetAuthorizedSkdeParamsResponse {
    fn from(params: SkdeParams) -> Self {
        Self {
            skde_params: params,
        }
    }
}

impl GetAuthorizedSkdeParamsResponse {
    pub fn into_skde_params(self) -> SkdeParams {
        SkdeParams {
            t: self.skde_params.t,
            n: self.skde_params.n,
            g: self.skde_params.g,
            h: self.skde_params.h,
            max_sequencer_number: self.skde_params.max_sequencer_number,
        }
    }
}
