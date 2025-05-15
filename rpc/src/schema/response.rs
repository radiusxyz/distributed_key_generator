
use skde::delay_encryption::SkdeParams;
use dkg_types::SignedSkdeParams;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GetSkdeParamsResponse {
    pub signed_skde_params: SignedSkdeParams,
}

impl GetSkdeParamsResponse {
    pub fn into_skde_params(self) -> SkdeParams {
        self.signed_skde_params.params
    }
}
