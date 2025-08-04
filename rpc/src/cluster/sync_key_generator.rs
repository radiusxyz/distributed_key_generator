use crate::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use dkg_primitives::{Config, Operator, ActiveOperatorList};
use std::fmt::{Display, Debug};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncKeyGenerator<Address> {
    // signature: Signature, // TODO: Auth 
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address: Clone> From<SyncKeyGenerator<Address>> for Operator<Address> {
    fn from(value: SyncKeyGenerator<Address>) -> Self {
        Operator::new(value.address, value.cluster_rpc_url, value.external_rpc_url)
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

    async fn handler(self, _ctx: C) -> RpcResult<Self::Response> {
        info!("Sync key generator - {}", self);
        let mut operators = ActiveOperatorList::<C::Address>::get_mut()?;
        if operators.contains(&self.address) {
            tracing::warn!("Already synced key generator: {}", self);
            return Ok(());
        }

        // TODO: Auth
        // self.signature.verify_signature(
        //     serialize_to_bincode(&self.message)?.as_slice(),
        //     context.config().radius_foundation_address().as_slice(),
        //     context.config().chain_type().clone(),
        // )?;

        operators.insert(self.into());
        operators.update()?;

        Ok(())
    }
}
