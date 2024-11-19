use std::sync::Arc;

use radius_sdk::json_rpc::server::{RpcError, RpcParameter};
use serde::{Deserialize, Serialize};

use crate::{state::AppState, types::KeyGeneratorList};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorList;

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

impl GetKeyGeneratorList {
    pub const METHOD_NAME: &'static str = "get_key_generator_list";

    pub async fn handler(
        _: RpcParameter,
        _: Arc<AppState>,
    ) -> Result<GetKeyGeneratorRpcUrlListResponse, RpcError> {
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
