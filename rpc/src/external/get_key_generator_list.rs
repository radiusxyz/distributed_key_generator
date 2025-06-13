use crate::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{Config, KeyGeneratorList, KeyGenerator, AddressT, DbManager};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorList;

// TODO: The `address` field inside `KeyGeneratorRpcInfo` must also be set to the authority's address.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyGeneratorRpcInfo {
    pub address: String,
    pub cluster_rpc_url: String,
    pub external_rpc_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
    pub urls: Vec<KeyGeneratorRpcInfo>,
}

impl<Address: AddressT> From<Response> for KeyGeneratorList<Address> {
    fn from(value: Response) -> Self {
        let mut key_generator_list = KeyGeneratorList::<Address>::new();
        let key_generator_rpc_url_list = value.urls;
        for key_generator_rpc_info in key_generator_rpc_url_list {
            key_generator_list.insert(KeyGenerator::new(key_generator_rpc_info.address.into(), key_generator_rpc_info.cluster_rpc_url, key_generator_rpc_info.external_rpc_url));
        }
        key_generator_list
    }
}

impl<C: Config> RpcParameter<C> for GetKeyGeneratorList {
    type Response = Response;

    fn method() -> &'static str {
        "get_key_generator_list"
    }

    async fn handler(self, ctx: C) -> Result<Self::Response, RpcError> {
        let current_round = ctx.db_manager().current_round().map_err(|e| RpcError::from(e))?;
        let key_generator_list = KeyGeneratorList::<C::Address>::get(current_round)?;

        let urls: Vec<KeyGeneratorRpcInfo> = key_generator_list
            .into_iter()
            .filter_map(|key_generator| {
                Some(KeyGeneratorRpcInfo {
                    address: key_generator.address().into(),
                    external_rpc_url: key_generator.external_rpc_url().to_owned(),
                    cluster_rpc_url: key_generator.cluster_rpc_url().to_owned(),
                })
            })
            .collect();
        Ok(Response { urls })
    }
}
