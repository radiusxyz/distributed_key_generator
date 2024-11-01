use std::sync::Arc;

use radius_sdk::json_rpc::server::{RpcError, RpcParameter};
use serde::{Deserialize, Serialize};

use crate::{
    error::{self, Error},
    state::AppState,
    types::{
        DistributedKeyGeneration, DistributedKeyGenerationAddressListModel,
        DistributedKeyGenerationModel, KeyGeneratorList,
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorList;

impl GetKeyGeneratorList {
    pub const METHOD_NAME: &'static str = "get_key_generator_list";

    pub async fn handler(_: RpcParameter, _: Arc<AppState>) -> Result<KeyGeneratorList, RpcError> {
        let key_generator_address_list = DistributedKeyGenerationAddressListModel::get()?;

        let key_generator_list = key_generator_address_list
            .iter()
            .map(
                |key_generator_address| -> Result<DistributedKeyGeneration, Error> {
                    DistributedKeyGenerationModel::get(key_generator_address)
                        .map_err(error::Error::Database)
                },
            )
            .collect::<Result<KeyGeneratorList, Error>>()?;

        Ok(key_generator_list)
    }
}
