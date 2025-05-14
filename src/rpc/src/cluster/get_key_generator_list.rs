use std::collections::HashSet;

use crate::primitives::*;
use dkg_types::{KeyGenerator, KeyGeneratorList};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorList;

// TODO: The `address` field inside `KeyGeneratorRpcInfo` must also be set to the authority's address.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyGeneratorRpcInfo {
    pub address: String,
    pub cluster_rpc_url: String,
    pub external_rpc_url: String,
}

impl From<KeyGeneratorRpcInfo> for KeyGenerator {
    fn from(info: KeyGeneratorRpcInfo) -> Self {
        let decoded_bytes =
            const_hex::decode(&info.address).expect("Invalid hex string in KeyGeneratorRpcInfo");
        let address = Address::from(decoded_bytes);
        Self::new(address, info.cluster_rpc_url, info.external_rpc_url)
    }
}

impl From<Vec<KeyGeneratorRpcInfo>> for KeyGeneratorList {
    fn from(info_vec: Vec<KeyGeneratorRpcInfo>) -> Self {
        let set: HashSet<KeyGenerator> = info_vec.into_iter().map(Into::into).collect();
        Self(set)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorRpcUrlListResponse {
    pub key_generator_rpc_url_list: Vec<KeyGeneratorRpcInfo>,
}

impl RpcParameter<AppState> for GetKeyGeneratorList {
    type Response = GetKeyGeneratorRpcUrlListResponse;

    fn method() -> &'static str {
        "get_key_generator_list"
    }

    async fn handler(self, _context: AppState) -> Result<Self::Response, RpcError> {
        let key_generator_list = KeyGeneratorList::get()?;

        let key_generator_rpc_url_list: Vec<KeyGeneratorRpcInfo> = key_generator_list
            .iter()
            .filter_map(|key_generator| {
                Some(KeyGeneratorRpcInfo {
                    address: key_generator.address().as_hex_string(),
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
