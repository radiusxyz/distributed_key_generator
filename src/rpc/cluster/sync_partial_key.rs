use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{
    verify_partial_key_validity, PartialKey as SkdePartialKey, PartialKeyProof,
};

use crate::rpc::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey {
    pub address: Address,
    pub session_id: SessionId,
    pub skde_partial_key: SkdePartialKey,
    pub partial_key_proof: PartialKeyProof,
}

impl RpcParameter<AppState> for SyncPartialKey {
    type Response = ();

    fn method() -> &'static str {
        "sync_partial_key"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        if KeyGeneratorList::get()?.is_key_generator_in_cluster(&self.address) {
            tracing::info!(
                "Sync partial key - key_id: {:?}, address: {:?}",
                self.session_id,
                self.address.as_hex_string(),
            );

            PartialKeyAddressList::initialize(self.session_id)?;

            let is_valid = verify_partial_key_validity(
                context.skde_params(),
                self.skde_partial_key.clone(),
                self.partial_key_proof,
            )
            .unwrap();

            if !is_valid {
                return Ok(());
            }

            PartialKeyAddressList::apply(self.session_id, |list| {
                list.insert(self.address.clone());
            })?;

            let partial_key = PartialKey::new(self.skde_partial_key.clone());
            partial_key.put(self.session_id, &self.address)?;
        }

        Ok(())
    }
}
