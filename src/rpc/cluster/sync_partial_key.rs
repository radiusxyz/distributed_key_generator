use std::sync::Arc;

use radius_sequencer_sdk::json_rpc::{types::RpcParameter, RpcError};
use serde::{Deserialize, Serialize};
use skde::key_generation::{verify_partial_key_validity, PartialKey, PartialKeyProof};
use tracing::info;

use crate::{
    state::AppState,
    types::{Address, KeyGeneratorModel, PartialKeyListModel},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKey {
    pub address: Address,
    pub key_id: u64,
    pub partial_key: PartialKey,
    pub partial_key_proof: PartialKeyProof,
}

impl SyncPartialKey {
    pub const METHOD_NAME: &'static str = "sync_partial_key";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;
        let is_key_generator_in_cluster = !KeyGeneratorModel::get(&parameter.address).is_err();

        if is_key_generator_in_cluster {
            info!(
                "Sync partial key - address: {:?}, partial key: {:?}",
                parameter.address, parameter.partial_key
            );

            PartialKeyListModel::initialize(parameter.key_id)?;

            let is_valid = verify_partial_key_validity(
                context.skde_params(),
                parameter.partial_key.clone(),
                parameter.partial_key_proof,
            );

            // TODO: Error handling
            if !is_valid {
                return Ok(());
            }

            PartialKeyListModel::add_key_generator_address(
                parameter.key_id,
                parameter.address,
                parameter.partial_key,
            )?;
        }

        Ok(())
    }
}
