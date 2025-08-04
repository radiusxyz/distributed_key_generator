use crate::Args;
use dkg_node_primitives::config::{
    DEFAULT_INTERNAL_RPC_URL, DEFAULT_EXTERNAL_RPC_URL, DEFAULT_CLUSTER_RPC_URL, DEFAULT_AUTHORITY_RPC_URL,
    DEFAULT_SOLVER_RPC_URL,
};

#[derive(Debug, Args)]
pub struct RpcServerArgs {
    #[arg(long = "internal.addr")]
    pub internal_rpc_url: Option<String>,
    #[arg(long = "external.addr")]
    pub external_rpc_url: Option<String>,
    #[arg(long = "cluster.addr")]
    pub cluster_rpc_url: Option<String>,
    /// Args for leader node
    #[arg(long = "authority.rpc.url")]
    pub authority_rpc_url: Option<String>,
    /// Args for solver node
    #[arg(long = "solver.rpc.url")]
    pub solver_rpc_url: Option<String>,
}

impl Default for RpcServerArgs {
    fn default() -> Self {
        Self {
            internal_rpc_url: Some(DEFAULT_INTERNAL_RPC_URL.to_string()),
            external_rpc_url: Some(DEFAULT_EXTERNAL_RPC_URL.to_string()),
            cluster_rpc_url: Some(DEFAULT_CLUSTER_RPC_URL.to_string()),
            authority_rpc_url: Some(DEFAULT_AUTHORITY_RPC_URL.to_string()),
            solver_rpc_url: Some(DEFAULT_SOLVER_RPC_URL.to_string()),
        }
    }
}