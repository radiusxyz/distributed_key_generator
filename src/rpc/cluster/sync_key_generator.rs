use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    state::AppState,
    types::{KeyGenerator, KeyGeneratorList},
    utils::log::log_prefix_role_and_address,
};

// TODO: The `signature` field must contain a valid signature from the authority who has the right to manage the KeyGenerator set.
// This ensures that only authorized entities can synchronize key generator information across the cluster.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator {
    signature: Signature,
    message: SyncKeyGeneratorMessage,
}

// TODO: The `address` field inside `SyncKeyGeneratorMessage` must also be set to the authority's address.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct SyncKeyGeneratorMessage {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl RpcParameter<AppState> for SyncKeyGenerator {
    type Response = ();

    fn method() -> &'static str {
        "sync_key_generator"
    }

    async fn handler(self, context: AppState) -> Result<Self::Response, RpcError> {
        let prefix = log_prefix_role_and_address(&context.config());
        info!(
            "{} Sync key generator - address: {:?} / cluster_rpc_url: {:?} / external_rpc_url: {:?}",
            prefix,
            self.message.address.as_hex_string(),
            self.message.cluster_rpc_url,
            self.message.external_rpc_url
        );

        let key_generator = KeyGenerator::new(
            self.message.address.clone(),
            self.message.cluster_rpc_url.clone(),
            self.message.external_rpc_url.clone(),
        );

        let key_generator_list = KeyGeneratorList::get()?;
        if key_generator_list.contains(&key_generator) {
            return Ok(());
        }

        KeyGeneratorList::apply(|key_generator_list| {
            key_generator_list.insert(key_generator);
        })?;

        Ok(())
    }
}
