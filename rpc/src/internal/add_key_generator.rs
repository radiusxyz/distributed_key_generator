use crate::{primitives::*, cluster::SyncKeyGenerator};
use dkg_primitives::{AppState, KeyGenerator, KeyGeneratorList};
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use std::fmt::{Debug, Display};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator<Address> {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address: Debug> Display for AddKeyGenerator<Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address: {:?}, cluster_rpc_url: {:?}, external_rpc_url: {:?}", self.address, self.cluster_rpc_url, self.external_rpc_url)
    }
}

impl<Address: Clone> From<AddKeyGenerator<Address>> for KeyGenerator<Address> {
    fn from(value: AddKeyGenerator<Address>) -> Self {
        KeyGenerator::new(value.address, value.cluster_rpc_url, value.external_rpc_url)
    }
}

// TODO (Post-PoC): Replace leader self-RPC calls for partial key submission and decryption key sync with direct internal handling.
// See Issue #38
impl<C: AppState> RpcParameter<C> for AddKeyGenerator<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "add_key_generator"
    }

    async fn handler(self, context: C) -> Result<Self::Response, RpcError> {
        let prefix = context.log_prefix();
        let key_generator_list = KeyGeneratorList::<C::Address>::get()?;
        if key_generator_list
            .into_iter()
            .any(|kg| kg.address() == self.address)
        {
            warn!("[{}] - Duplicate key generator registration for {}", prefix, self);
            return Ok(());
        }

        info!("[{}] - Add distributed key generation for {}", prefix, self);

        KeyGeneratorList::apply(|key_generator_list| {
            key_generator_list.insert(self.clone().into());
        })?;

        sync_key_generator::<C>(self);

        Ok(())
    }
}

pub fn sync_key_generator<C: AppState>(key_generator: AddKeyGenerator<C::Address>) {
    let key_generator_rpc_url_list = KeyGeneratorList::<C::Address>::get()
        .unwrap()
        .get_all_key_generator_rpc_url_list();

    tokio::spawn(async move {
        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                key_generator_rpc_url_list,
                <SyncKeyGenerator::<C::Address> as RpcParameter<C>>::method(),
                &key_generator,
                Id::Null,
            )
            .await
            .unwrap();
    });
}
