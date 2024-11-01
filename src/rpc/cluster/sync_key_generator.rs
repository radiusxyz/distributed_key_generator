use std::sync::Arc;

use radius_sdk::json_rpc::server::{RpcError, RpcParameter};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    state::AppState,
    types::{
        Address, DistributedKeyGeneration, DistributedKeyGenerationAddressListModel,
        DistributedKeyGenerationModel,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SyncKeyGeneratorMessage {
    address: Address,
    ip_address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator {
    // signature: Signature, // TODO: Uncomment this code
    message: SyncKeyGeneratorMessage,
}

impl SyncKeyGenerator {
    pub const METHOD_NAME: &'static str = "sync_key_generator";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        info!(
            "Sync key generator - address: {:?}, url: {:?}",
            parameter.message.address, parameter.message.ip_address
        );

        // TODO: Uncomment this code
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let key_generator_address_list = DistributedKeyGenerationAddressListModel::get()?;
        if key_generator_address_list.contains(&parameter.message.address) {
            return Ok(());
        }

        DistributedKeyGenerationAddressListModel::add_distributed_key_generation_address(
            parameter.message.address.clone(),
        )?;

        let key_generator =
            DistributedKeyGeneration::new(parameter.message.address, parameter.message.ip_address);
        DistributedKeyGenerationModel::put(&key_generator)?;

        context.add_key_generator_client(key_generator).await?;

        Ok(())
    }
}
