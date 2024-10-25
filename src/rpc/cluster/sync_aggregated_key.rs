use std::sync::Arc;

use radius_sequencer_sdk::json_rpc::{types::RpcParameter, RpcError};
use serde::{Deserialize, Serialize};
use skde::{
    delay_encryption::solve_time_lock_puzzle,
    key_aggregation::{aggregate_key, AggregatedKey},
};
use tracing::info;

use crate::{
    state::AppState,
    types::{Address, AggregatedKeyModel, DecryptionKeyModel, PartialKeyListModel},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncAggregatedKey {
    pub key_id: u64,
    // TODO: unused field
    pub aggregated_key: AggregatedKey,
    pub participant_addresses: Vec<Address>,
}

impl SyncAggregatedKey {
    pub const METHOD_NAME: &'static str = "sync_aggregated_key";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        info!(
            "Sync aggregated key - key_id: {:?}, participant address: {:?}",
            parameter.key_id, parameter.participant_addresses
        );

        let skde_params = context.skde_params().clone();

        let partial_key_list = PartialKeyListModel::get_or_default(parameter.key_id).unwrap();

        // TODO: validate
        let _participant_addresses = partial_key_list.get_address_list();

        let aggregated_key = aggregate_key(&skde_params, &partial_key_list.to_vec());
        AggregatedKeyModel::put(parameter.key_id, &aggregated_key).unwrap();

        info!("Aggregated key: {:?}", aggregated_key);

        tokio::spawn(async move {
            let decryption_key = solve_time_lock_puzzle(&skde_params, &aggregated_key).unwrap();
            DecryptionKeyModel::put(parameter.key_id, &decryption_key).unwrap();
            info!("Decryption key: {:?}", decryption_key);
        });

        Ok(())
    }
}
