pub mod rpc;
pub mod worker;
pub use worker::{DkgWorker, run_dkg_worker};

pub mod authority;
pub mod committee;
pub mod leader;
pub mod solver;

use dkg_primitives::{AppState, Error};
use dkg_node_primitives::Config;
use radius_sdk::json_rpc::server::{RpcParameter, RpcServer};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTrustedSetup<Signature, TrustedSetup> {
    pub trusted_setup: TrustedSetup,
    pub signature: Signature,
}