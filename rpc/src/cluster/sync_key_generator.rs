use crate::primitives::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{AppState, KeyGenerator, KeyGeneratorList};
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

impl<C: AppState> RpcParameter<C> for SyncKeyGenerator<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "sync_key_generator"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        info!("Sync key generator - {}", self);
        let mut key_generator_list = KeyGeneratorList::get_mut()?;
        if key_generator_list.contains(&self.address) {
            tracing::warn!("Already synced key generator: {}", self);
            return Ok(());
        }

        // TODO: Auth
        // self.signature.verify_signature(
        //     serialize_to_bincode(&self.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        key_generator_list.insert(self.into());
        key_generator_list.update()?;

        Ok(())
    }
}
