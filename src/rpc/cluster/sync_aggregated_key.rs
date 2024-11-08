use std::sync::Arc;

use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::{
    delay_encryption::solve_time_lock_puzzle,
    key_aggregation::{aggregate_key, AggregatedKey as SkdeAggregatedKey},
};
use tracing::info;

use crate::{state::AppState, types::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncAggregatedKey {
    pub key_id: KeyId,
    pub aggregated_key: SkdeAggregatedKey,
    pub participant_addresses: Vec<Address>,
}

impl SyncAggregatedKey {
    pub const METHOD_NAME: &'static str = "sync_aggregated_key";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let skde_params = context.skde_params().clone();

        let partial_key_address_list =
            PartialKeyAddressList::get_or(parameter.key_id, PartialKeyAddressList::default)?;

        let partial_key_list = partial_key_address_list.get_partial_key_list(parameter.key_id)?;

        let skde_aggregated_key = aggregate_key(&skde_params, &partial_key_list);
        let aggregated_key = AggregatedKey::new(skde_aggregated_key.clone());
        aggregated_key.put(parameter.key_id)?;

        info!(
            "Completed to generate encryption key - key id: {:?} / encryption key: {:?}",
            parameter.key_id, skde_aggregated_key.u
        );

        tokio::spawn(async move {
            let decryption_key =
                solve_time_lock_puzzle(&skde_params, &skde_aggregated_key).unwrap();
            let decryption_key = DecryptionKey::new(decryption_key.sk.clone());

            decryption_key.put(parameter.key_id).unwrap();
            info!(
                "Complete to get decryption key - key_id: {:?} / decryption key: {:?}",
                parameter.key_id, decryption_key
            );
        });

        Ok(())
    }
}
