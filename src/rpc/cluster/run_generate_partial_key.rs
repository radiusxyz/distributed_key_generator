use std::{collections::BTreeMap, sync::Arc};

use radius_sequencer_sdk::json_rpc::{types::RpcParameter, RpcError};
use serde::{Deserialize, Serialize};
use skde::key_generation::{
    generate_partial_key, prove_partial_key_validity, PartialKey, PartialKeyProof,
};
use tracing::info;

use crate::{
    client::key_generator::KeyGeneratorClient, rpc::cluster::SyncPartialKey, state::AppState,
    types::Address,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunGeneratePartialKey {
    pub key_id: u64,
}

impl RunGeneratePartialKey {
    pub const METHOD_NAME: &'static str = "run_generate_partial_key";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let skde_params = context.skde_params();

        let (secret_value, partial_key) = generate_partial_key(skde_params);
        let partial_key_proof = prove_partial_key_validity(skde_params, &secret_value);

        let key_generator_clients = context.key_generator_clients().await.unwrap();

        sync_partial_key(
            key_generator_clients,
            context.config().signing_key().get_address().clone(),
            parameter.key_id,
            partial_key,
            partial_key_proof,
        );

        Ok(())
    }
}

pub fn sync_partial_key(
    key_generator_clients: BTreeMap<Address, KeyGeneratorClient>,
    address: Address,
    key_id: u64,
    partial_key: PartialKey,
    partial_key_proof: PartialKeyProof,
) {
    tokio::spawn(async move {
        let parameter = SyncPartialKey {
            address,
            key_id,
            partial_key,
            partial_key_proof,
        };

        info!(
            "sync_partial_key - rpc_client_count: {:?}",
            key_generator_clients.len()
        );

        for (_address, key_generator_rpc_client) in key_generator_clients {
            let key_generator_rpc_client = key_generator_rpc_client.clone();
            let parameter = parameter.clone();

            tokio::spawn(async move {
                match key_generator_rpc_client.sync_partial_key(parameter).await {
                    Ok(_) => {
                        info!("Complete to sync partial key");
                    }
                    Err(err) => {
                        info!("Failed to sync partial key - error: {:?}", err);
                    }
                }
            });
        }
    });
}
