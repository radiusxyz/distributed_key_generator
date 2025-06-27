mod dkg;
mod rpc_server;
mod data_dir;

use dkg::DkgArgs;
use rpc_server::RpcServerArgs;
use data_dir::DataDirArgs;
use crate::Parser;

#[derive(Debug, Parser)]
pub struct NodeCommand {
    #[arg(long = "dev")]
    pub is_dev: bool,
    #[arg(long = "node-name")]
    pub node_name: Option<String>,
    #[command(flatten)]
    pub rpc: RpcServerArgs,
    #[command(flatten)]
    pub dkg: DkgArgs,
    #[command(flatten)]
    pub data_dir: DataDirArgs,
}