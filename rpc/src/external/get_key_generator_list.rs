use dkg_primitives::{ActiveOperatorList, AddressT, Config, Operator};
use serde::{Deserialize, Serialize};

use crate::*;

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

impl<Address: AddressT> From<Response> for ActiveOperatorList<Address> {
    fn from(value: Response) -> Self {
        let mut operator_list = ActiveOperatorList::<Address>::new();
        let operator_rpc_url_list = value.urls;
        for operator_rpc_info in operator_rpc_url_list {
            operator_list.insert(Operator::new(
                operator_rpc_info.address.into(),
                operator_rpc_info.cluster_rpc_url,
                operator_rpc_info.external_rpc_url,
            ));
        }
        operator_list
    }
}

impl<C: Config> RpcParameter<C> for GetKeyGeneratorList {
    type Response = Response;

    fn method() -> &'static str {
        "get_key_generator_list"
    }

    async fn handler(self, _ctx: C) -> Result<Self::Response, RpcError> {
        let operator_list = ActiveOperatorList::<C::Address>::get()?;

        let urls: Vec<KeyGeneratorRpcInfo> = operator_list
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
