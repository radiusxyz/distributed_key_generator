pub mod rpc;
mod worker;
pub use worker::*;

pub mod committee;
pub mod solver;

use dkg_primitives::Config;
use dkg_node_primitives::NodeConfig;
use radius_sdk::json_rpc::server::RpcParameter;