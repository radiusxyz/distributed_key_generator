use crate::primitives::*;
use dkg_primitives::{AppState, TrustedSetupFor, SecureBlock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// RPC method for getting the trusted setup(e.g SKDE params)
pub struct GetTrustedSetup;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response<Signature, TrustedSetup> {
    pub trusted_setup: TrustedSetup,
    pub signature: Signature,
}

impl<C: AppState> RpcParameter<C> for GetTrustedSetup {
    type Response = Response<C::Signature, TrustedSetupFor<C>>;

    fn method() -> &'static str {
        "get_trusted_setup"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let trusted_setup = context.secure_block().get_trusted_setup();
        let signature = context.sign(&trusted_setup)?;
        Ok(Response { trusted_setup, signature })
    }
}
