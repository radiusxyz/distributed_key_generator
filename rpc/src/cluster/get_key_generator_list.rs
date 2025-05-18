use crate::primitives::*;
use serde::{Deserialize, Serialize};
use dkg_primitives::{AppState, KeyGeneratorList};

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
pub struct GetKeyGeneratorRpcUrlListResponse {
    pub key_generator_rpc_url_list: Vec<KeyGeneratorRpcInfo>,
}

impl<C> RpcParameter<C> for GetKeyGeneratorList 
where
    C: AppState + 'static,
    C::Address: Clone + Into<String>,
{
    type Response = GetKeyGeneratorRpcUrlListResponse;

    fn method() -> &'static str {
        "get_key_generator_list"
    }

    async fn handler(self, _context: C) -> Result<Self::Response, RpcError> {
        let key_generator_list = KeyGeneratorList::<C::Address>::get()?;

        let key_generator_rpc_url_list: Vec<KeyGeneratorRpcInfo> = key_generator_list
            .into_iter()
            .filter_map(|key_generator| {
                Some(KeyGeneratorRpcInfo {
                    address: key_generator.address().into(),
                    external_rpc_url: key_generator.external_rpc_url().to_owned(),
                    cluster_rpc_url: key_generator.cluster_rpc_url().to_owned(),
                })
            })
            .collect();

        Ok(GetKeyGeneratorRpcUrlListResponse {
            key_generator_rpc_url_list,
        })
    }
}
