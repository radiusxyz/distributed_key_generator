use std::{collections::BTreeMap, time::Duration};

use skde::{
    delay_encryption::solve_time_lock_puzzle,
    key_aggregation::{aggregate_key, AggregatedKey},
};
use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    client::key_generator::KeyGeneratorClient,
    rpc::cluster::{RunGeneratePartialKey, SyncAggregatedKey},
    state::AppState,
    types::{Address, AggregatedKeyModel, DecryptionKeyModel, KeyIdModel, PartialKeyListModel},
};

pub fn run_single_key_generator(context: AppState) {
    tokio::spawn(async move {
        let partial_key_generation_cycle = context.config().partial_key_generation_cycle();
        let partial_key_aggregation_cycle = context.config().partial_key_aggregation_cycle();

        loop {
            sleep(Duration::from_secs(partial_key_generation_cycle)).await;
            let key_generator_clients = context.key_generator_clients().await.unwrap();
            let context = context.clone();

            let key_id = KeyIdModel::get().unwrap();

            info!("request run_generate_partial_key - key_id: {:?}", key_id);
            run_generate_partial_key(key_generator_clients.clone(), key_id);

            tokio::spawn(async move {
                sleep(Duration::from_secs(partial_key_aggregation_cycle)).await;
                let skde_params = context.skde_params().clone();

                let partial_key_list = PartialKeyListModel::get_or_default(key_id).unwrap();

                let participant_addresses = partial_key_list.get_address_list();

                let aggregated_key = aggregate_key(&skde_params, &partial_key_list.to_vec());
                AggregatedKeyModel::put(key_id, &aggregated_key).unwrap();

                info!("Aggregated key: {:?}", aggregated_key);

                sync_aggregated_key(
                    key_generator_clients,
                    key_id,
                    aggregated_key.clone(),
                    participant_addresses,
                );

                let decryption_key = solve_time_lock_puzzle(&skde_params, &aggregated_key).unwrap();
                DecryptionKeyModel::put(key_id, &decryption_key).unwrap();
                info!("Decryption key: {:?}", decryption_key);
            });

            KeyIdModel::increase_key_id().unwrap();
        }
    });
}

pub fn run_generate_partial_key(
    key_generator_clients: BTreeMap<Address, KeyGeneratorClient>,
    key_id: u64,
) {
    tokio::spawn(async move {
        let parameter = RunGeneratePartialKey { key_id };

        info!(
            "run_generate_partial_key - rpc_client_count: {:?}",
            key_generator_clients.len()
        );

        for (_address, key_generator_rpc_client) in key_generator_clients {
            let key_generator_rpc_client = key_generator_rpc_client.clone();
            let parameter = parameter.clone();

            tokio::spawn(async move {
                match key_generator_rpc_client
                    .run_generate_partial_key(parameter)
                    .await
                {
                    Ok(_) => {
                        info!("Complete to run generate partial key");
                    }
                    Err(err) => {
                        error!("Failed to run generate partial key - error: {:?}", err);
                    }
                }
            });
        }
    });
}

pub fn sync_aggregated_key(
    key_generator_clients: BTreeMap<Address, KeyGeneratorClient>,
    key_id: u64,
    aggregated_key: AggregatedKey,
    participant_addresses: Vec<Address>,
) {
    tokio::spawn(async move {
        let parameter = SyncAggregatedKey {
            key_id,
            aggregated_key,
            participant_addresses,
        };

        info!(
            "sync_aggregated_key - rpc_client_count: {:?}",
            key_generator_clients.len()
        );

        for (_address, key_generator_rpc_client) in key_generator_clients {
            let key_generator_rpc_client = key_generator_rpc_client.clone();
            let parameter = parameter.clone();

            tokio::spawn(async move {
                match key_generator_rpc_client
                    .sync_aggregated_key(parameter)
                    .await
                {
                    Ok(_) => {
                        info!("Complete to sync aggregated key");
                    }
                    Err(err) => {
                        error!("Failed to sync aggregated key - error: {:?}", err);
                    }
                }
            });
        }
    });
}
