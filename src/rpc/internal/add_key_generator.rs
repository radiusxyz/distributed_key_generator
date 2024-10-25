use std::collections::BTreeMap;

use tracing::info;

use crate::{client::key_generator::KeyGeneratorClient, rpc::prelude::*};

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
        info!(
            "Add key generator - address: {:?} , url: {:?}",
            parameter.message.address, parameter.message.ip_address
        );

        // TODO: Uncomment this code
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let key_generator_address_list = KeyGeneratorAddressListModel::get()?;
        if key_generator_address_list.contains(&parameter.message.address) {
            return Ok(());
        }

        KeyGeneratorAddressListModel::add_key_generator_address(parameter.message.address.clone())?;

        let key_generator = KeyGenerator::new(
            parameter.message.address.clone(),
            parameter.message.ip_address.clone(),
        );
        KeyGeneratorModel::put(&key_generator)?;

        context.add_key_generator_client(key_generator).await?;

        let key_generator_clients = context.key_generator_clients().await?;

        sync_key_generator(key_generator_clients, parameter);

        Ok(())
    }
}

pub fn sync_key_generator(
    key_generator_clients: BTreeMap<Address, KeyGeneratorClient>,
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
