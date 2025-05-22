use crate::primitives::*;
use dkg_primitives::AppState;
use serde::{Deserialize, Serialize};
use skde::delay_encryption::SkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetSkdeParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response<Signature> {
    pub skde_params: SkdeParams,
    pub signature: Signature,
}

impl<C: AppState> RpcParameter<C> for GetSkdeParams {
    type Response = Response<C::Signature>;

    fn method() -> &'static str {
        "get_skde_params"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let skde_params = context.skde_params();
        let signature = context.sign(&skde_params)?;
        Ok(Response { skde_params, signature })
    }
}
