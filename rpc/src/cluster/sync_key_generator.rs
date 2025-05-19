use crate::primitives::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{AppState, KeyGenerator, KeyGeneratorList};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator<Address> {
    // signature: Signature, // TODO: Uncomment this code
    message: SyncKeyGeneratorMessage<Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SyncKeyGeneratorMessage<Address> {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<C: AppState> RpcParameter<C> for SyncKeyGenerator<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_key_generator"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();
        info!(
            "{} Sync key generator - address: {:?} / cluster_rpc_url: {:?} / external_rpc_url: {:?}",
            prefix,
            self.message.address,
            self.message.cluster_rpc_url,
            self.message.external_rpc_url
        );

        // TODO: Uncomment this code
        // self.signature.verify_signature(
        //     serialize_to_bincode(&self.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        let key_generator = KeyGenerator::new(
            self.message.address,
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
