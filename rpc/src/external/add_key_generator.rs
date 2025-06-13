use crate::{*, cluster::SyncKeyGenerator};
use dkg_primitives::{Config, KeyGenerator, KeyGeneratorList, AsyncTask, DbManager};
use serde::{Serialize, Deserialize};
use std::fmt::{Debug, Display};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator<Address> {
    is_solver: bool,
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

impl<Address> AddKeyGenerator<Address> {
    pub fn new(is_solver: bool, address: Address, cluster_rpc_url: String, external_rpc_url: String) -> Self {
        Self { is_solver, address, cluster_rpc_url, external_rpc_url }
    }
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

// TODO: Replace leader self-RPC calls for encryption key submission and decryption key sync with direct internal handling(Issue #38
impl<C: Config> RpcParameter<C> for AddKeyGenerator<C::Address> {
    type Response = ();

    fn method() -> &'static str {
        "add_key_generator"
    }

    async fn handler(self, ctx: C) -> RpcResult<Self::Response> {
        let mut urls = Vec::new();
        if !self.is_solver {
            let current_round = ctx.db_manager().current_round().map_err(|e| RpcError::from(e))?;
            KeyGeneratorList::<C::Address>::apply(current_round, |new| { 
                if !new.into_iter().any(|kg| kg.address() == self.address) {
                    new.insert(self.clone().into()); 
                    urls = new.all_rpc_urls(true); // All cluster RPC urls
                }
            })?;
        }

        ctx.async_task().multicast(urls, <SyncKeyGenerator::<C::Address> as RpcParameter<C>>::method().into(), self);

        Ok(())
    }
}
