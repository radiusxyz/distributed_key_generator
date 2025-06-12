use crate::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{Config, KeyGenerator, KeyGeneratorList};
use std::fmt::{Display, Debug};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator<Address> {
    // signature: Signature, // TODO: Auth 
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address: Clone> From<SyncKeyGenerator<Address>> for KeyGenerator<Address> {
    fn from(value: SyncKeyGenerator<Address>) -> Self {
        KeyGenerator::new(value.address, value.cluster_rpc_url, value.external_rpc_url)
    }
}

impl<Address: Debug> Display for SyncKeyGenerator<Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address: {:?}, cluster_rpc_url: {:?}, external_rpc_url: {:?}", self.address, self.cluster_rpc_url, self.external_rpc_url)
    }
}

impl<C: Config> RpcParameter<C> for SyncKeyGenerator<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_key_generator"
    }

    async fn handler(self, _context: C) -> RpcResult<Self::Response> {
        info!("Sync key generator - {}", self);
        let current_round = _context.current_round().map_err(|e| C::Error::from(e))?;
        let mut key_generators = KeyGeneratorList::<C::Address>::get_mut(current_round)?;
        if key_generators.contains(&self.address) {
            tracing::warn!("Already synced key generator: {}", self);
            return Ok(());
        }

        // TODO: Auth
        // self.signature.verify_signature(
        //     serialize_to_bincode(&self.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        key_generators.insert(self.into());
        key_generators.update()?;

        Ok(())
    }
}
