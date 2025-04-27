use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

use crate::{rpc::prelude::*, task::authority_setup::SignedSkdeParams, utils::create_signature};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetAuthorizedSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl RpcParameter<AppState> for GetAuthorizedSkdeParams {
    type Response = GetAuthorizedSkdeParamsResponse;

    fn method() -> &'static str {
        "get_authorized_skde_params"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params().clone();

        let signature = create_signature(&skde_params);

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
// impl From<SkdeParams> for GetAuthorizedSkdeParamsResponse {
//     fn from(params: SkdeParams) -> Self {
//         Self {
//             skde_params: params,
//         }
//     }
// }

// impl GetAuthorizedSkdeParamsResponse {
//     pub fn into_skde_params(self) -> SkdeParams {
//         SkdeParams {
//             t: self.skde_params.t,
//             n: self.skde_params.n,
//             g: self.skde_params.g,
//             h: self.skde_params.h,
//             max_sequencer_number: self.skde_params.max_sequencer_number,
//         }
//     }
// }
