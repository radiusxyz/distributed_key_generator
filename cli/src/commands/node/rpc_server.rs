use std::net::{IpAddr, Ipv4Addr};
use crate::Args;
use dkg_node_primitives::config::{
    DEFAULT_INTERNAL_RPC_PORT, DEFAULT_EXTERNAL_RPC_PORT, DEFAULT_CLUSTER_RPC_PORT, DEFAULT_LEADER_RPC_PORT, DEFAULT_AUTHORITY_RPC_PORT,
};

#[derive(Debug, Args)]
pub struct RpcServerArgs {
    #[arg(long = "internal.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub internal_rpc_url: IpAddr,
    #[arg(long = "internal.port", default_value_t = DEFAULT_INTERNAL_RPC_PORT)]
    pub internal_rpc_port: u16,
    #[arg(long = "external.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub external_rpc_url: IpAddr,
    #[arg(long = "external.port", default_value_t = DEFAULT_EXTERNAL_RPC_PORT)]
    pub external_rpc_port: u16,
    #[arg(long = "cluster.addr", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub cluster_rpc_url: IpAddr,
    #[arg(long = "cluster.port", default_value_t = DEFAULT_CLUSTER_RPC_PORT)]
    pub cluster_rpc_port: u16,
    #[arg(long = "leader.rpc.url")]
    pub leader_rpc_url: Option<String>,
}

impl Default for RpcServerArgs {
    fn default() -> Self {
        Self {
            internal_rpc_url: IpAddr::V4(Ipv4Addr::LOCALHOST).into(),
            internal_rpc_port: DEFAULT_INTERNAL_RPC_PORT,
            external_rpc_url: IpAddr::V4(Ipv4Addr::LOCALHOST).into(),
            external_rpc_port: DEFAULT_EXTERNAL_RPC_PORT,
            cluster_rpc_url: IpAddr::V4(Ipv4Addr::LOCALHOST).into(),
            cluster_rpc_port: DEFAULT_CLUSTER_RPC_PORT,
            leader_rpc_url: None,
        }
    }
}

impl RpcServerArgs {
    pub fn external_rpc_url(&self) -> String {
        format!("{}:{}", self.external_rpc_url, self.external_rpc_port)
    }

    pub fn internal_rpc_url(&self) -> String {
        format!("{}:{}", self.internal_rpc_url, self.internal_rpc_port)
    }

    pub fn cluster_rpc_url(&self) -> String {
        format!("{}:{}", self.cluster_rpc_url, self.cluster_rpc_port)
    }
}