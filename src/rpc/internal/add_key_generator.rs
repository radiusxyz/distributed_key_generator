use radius_sdk::{json_rpc::client::Id, signature::Address};
use tracing::info;

use crate::rpc::{cluster::SyncKeyGenerator, prelude::*};

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

    pub async fn handler(parameter: RpcParameter, _context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;
        info!(
            "Add distributed key generation - address: {:?} / url: {:?}",
            parameter.message.address.as_hex_string(),
            parameter.message.ip_address
        );

        // TODO: Uncomment this code
        // parameter.signature.verify_signature(
        //     serialize_to_bincode(&parameter.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let key_generator = KeyGenerator::new(
            parameter.message.address.clone(),
            parameter.message.ip_address.clone(),
        );

        let key_generator_address_list = KeyGeneratorList::get()?;
        if key_generator_address_list.contains(&key_generator) {
            return Ok(());
        }

        KeyGeneratorList::apply(|key_generator_list| {
            key_generator_list.insert(key_generator);
        })?;

        sync_key_generator(parameter);

        Ok(())
    }
}

pub fn sync_key_generator(parameter: AddKeyGenerator) {
    let other_key_generator_rpc_url_list = KeyGeneratorList::get()
        .unwrap()
        .get_all_key_generator_rpc_url_list();

    tokio::spawn(async move {
        info!(
            "Sync distributed key generation - address: {:?} / ip_address: {:?} / rpc_client_count: {:?}",
            parameter.message.address.as_hex_string(),
            parameter.message.ip_address,
            other_key_generator_rpc_url_list.len()
        );

        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                other_key_generator_rpc_url_list,
                SyncKeyGenerator::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await;
    });
}
