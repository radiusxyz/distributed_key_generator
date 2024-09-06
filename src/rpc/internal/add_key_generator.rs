use std::{collections::HashMap, sync::Arc};

use radius_sequencer_sdk::{
    json_rpc::{types::RpcParameter, RpcError},
    signature::Signature,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    client::key_generator::KeyGeneratorClient,
    models::{KeyGeneratorAddressListModel, KeyGeneratorModel},
    state::AppState,
    types::{Address, KeyGenerator},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AddKeyGeneratorMessage {
    address: Address,
    ip_address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator {
    // signature: Signature, // TODO: Uncomment this code
    message: AddKeyGeneratorMessage,
}

impl AddKeyGenerator {
    pub const METHOD_NAME: &'static str = "add_key_generator";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        // TODO: Uncomment this code
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let mut key_generator_address_list = KeyGeneratorAddressListModel::get()?;
        key_generator_address_list.insert(parameter.message.address.clone());

        KeyGeneratorAddressListModel::put(&key_generator_address_list)?;

        let key_generator =
            KeyGenerator::new(parameter.message.address, parameter.message.ip_address);
        KeyGeneratorModel::put(&key_generator)?;

        // let key_generator_clients = context.key_generator_clients().await?;

        context.add_key_generator_client(key_generator).await?;

        Ok(())
    }
}

pub fn sync_key_generator(
    key_generator_clients: HashMap<Address, KeyGeneratorClient>,
    parameter: AddKeyGenerator,
) {
    tokio::spawn(async move {
        info!(
            "sync key generator: {:?} / rpc_client_count: {:?}",
            parameter,
            key_generator_clients.len()
        );

        for (_, key_generator_client) in key_generator_clients {
            let parameter = parameter.clone();

            tokio::spawn(async move {
                let _ = key_generator_client.sync_key_generator(parameter).await;
            });
        }
    });
}
