use std::sync::Arc;

use radius_sdk::{
    json_rpc::server::{RpcError, RpcParameter},
    signature::Address,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    state::AppState,
    types::{KeyGenerator, KeyGeneratorList},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SyncKeyGeneratorMessage {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator {
    // signature: Signature, // TODO: Uncomment this code
    message: SyncKeyGeneratorMessage,
}

impl SyncKeyGenerator {
    pub const METHOD_NAME: &'static str = "sync_key_generator";

    pub async fn handler(parameter: RpcParameter, _context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        info!(
            "Sync key generator - address: {:?} / cluster_rpc_url: {:?} / external_rpc_url: {:?}",
            parameter.message.address.as_hex_string(),
            parameter.message.cluster_rpc_url,
            parameter.message.external_rpc_url
        );

        // TODO: Uncomment this code
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let key_generator = KeyGenerator::new(
            parameter.message.address.clone(),
            parameter.message.cluster_rpc_url.clone(),
            parameter.message.external_rpc_url.clone(),
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
