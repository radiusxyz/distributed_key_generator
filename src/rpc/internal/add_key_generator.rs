use radius_sdk::signature::{Address, Signature};
use tracing::{info, warn};

use crate::{
    error::KeyGenerationError,
    rpc::{cluster::SyncKeyGenerator, prelude::*},
    utils::signature::verify_signature,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator {
    signature: Signature,
    message: AddKeyGeneratorMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGeneratorMessage {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

// TODO (Post-PoC): Replace leader self-RPC calls for partial key submission and decryption key sync with direct internal handling.
// See Issue #38
impl RpcParameter<AppState> for AddKeyGenerator {
    type Response = ();

    fn method() -> &'static str {
        "add_key_generator"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let signer = verify_signature(&self.signature, &self.message)?;
        if &signer != &self.message.address {
            return Err(RpcError::from(KeyGenerationError::InvalidPartialKey(
                "Signature does not match sender address".into(),
            )));
        }

        let key_generator_list = KeyGeneratorList::get()?;
        if key_generator_list
            .iter()
            .any(|kg| kg.address() == &self.message.address)
        {
            warn!(
                "Duplicate key generator registration - address: {:?} / cluster_rpc_url: {:?} / external_rpc_url: {:?}",
                self.message.address.as_hex_string(),
                self.message.cluster_rpc_url,
                self.message.external_rpc_url
            );
            return Ok(());
        }

        info!(
            "Add distributed key generation - address: {:?} / cluster_rpc_url: {:?} / external_rpc_url: {:?}",
            self.message.address.as_hex_string(),
            self.message.cluster_rpc_url,
            self.message.external_rpc_url
        );

        let key_generator = KeyGenerator::new(
            self.message.address.clone(),
            self.message.cluster_rpc_url.clone(),
            self.message.external_rpc_url.clone(),
        );

        KeyGeneratorList::apply(|key_generator_list| {
            key_generator_list.insert(key_generator);
        })?;

        sync_key_generator(self);

        Ok(())
    }
}

pub fn sync_key_generator(add_key_generator: AddKeyGenerator) {
    let key_generator_rpc_url_list = KeyGeneratorList::get()
        .unwrap()
        .get_all_key_generator_rpc_url_list();

    tokio::spawn(async move {
        info!(
            "Sync distributed key generation - address: {:?} / cluster_rpc_url: {:?} / rpc_client_count: {:?}",
            add_key_generator.message.address.as_hex_string(),
            add_key_generator.message.cluster_rpc_url,
            key_generator_rpc_url_list.len()
        );

        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                key_generator_rpc_url_list,
                SyncKeyGenerator::method(),
                &add_key_generator,
                Id::Null,
            )
            .await
            .unwrap();
    });
}
