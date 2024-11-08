use std::sync::Arc;

use radius_sdk::json_rpc::server::{RpcError, RpcParameter};
use serde::{Deserialize, Serialize};

use crate::{state::AppState, types::KeyGeneratorList};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetKeyGeneratorList;

impl GetKeyGeneratorList {
    pub const METHOD_NAME: &'static str = "get_key_generator_list";

    pub async fn handler(_: RpcParameter, _: Arc<AppState>) -> Result<KeyGeneratorList, RpcError> {
        let key_generator_list = KeyGeneratorList::get()?;

        Ok(key_generator_list)
    }
}
