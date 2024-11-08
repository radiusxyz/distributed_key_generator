use std::sync::Arc;

use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{
    verify_partial_key_validity, PartialKey as SkdePartialKey, PartialKeyProof,
};
use tracing::info;

use crate::{state::AppState, types::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey {
    pub address: Address,
    pub key_id: KeyId,
    pub skde_partial_key: SkdePartialKey,
    pub partial_key_proof: PartialKeyProof,
}

impl SyncPartialKey {
    pub const METHOD_NAME: &'static str = "sync_partial_key";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        if KeyGeneratorList::get()?.is_key_generator_in_cluster(&parameter.address) {
            info!(
                "Sync partial key - key_id: {:?}, address: {:?}",
                parameter.key_id,
                parameter.address.as_hex_string(),
            );

            PartialKeyAddressList::initialize(parameter.key_id)?;

            let is_valid = verify_partial_key_validity(
                context.skde_params(),
                parameter.skde_partial_key.clone(),
                parameter.partial_key_proof,
            );

            if !is_valid {
                return Ok(());
            }

            PartialKeyAddressList::apply(parameter.key_id, |list| {
                list.insert(parameter.address.clone());
            })?;

            let partial_key = PartialKey::new(parameter.skde_partial_key.clone());
            partial_key.put(parameter.key_id, &parameter.address)?;
        }

        Ok(())
    }
}
