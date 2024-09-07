use std::sync::Arc;

use radius_sequencer_sdk::json_rpc::{types::RpcParameter, RpcError};
use serde::{Deserialize, Serialize};

use crate::{state::AppState, task::single_key_generator::run_single_key_generator};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunSingleKeyGenerator {
    key_id: u64,
}

impl RunSingleKeyGenerator {
    pub const METHOD_NAME: &'static str = "run_single_key_generator";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        run_single_key_generator(context, parameter.key_id);

        Ok(())
    }
}
