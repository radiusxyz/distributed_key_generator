pub mod cluster;
pub mod debug;
pub mod external;
pub mod internal;
pub mod prelude {
    pub use std::sync::Arc;

    pub use radius_sequencer_sdk::json_rpc::{types::*, RpcClient, RpcError};
    pub use serde::{Deserialize, Serialize};

    pub use crate::{error::Error, state::AppState, types::*};
}

pub mod methods {
    pub fn serialize_to_bincode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(value)
    }
}
