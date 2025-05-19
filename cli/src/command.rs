use crate::node::NodeCommand;
use dkg_node_primitives::Config;
use dkg_primitives::Error;
use crate::{Cli, Commands};

pub fn run() -> Result<(), Error> {
    let cli = Cli::init();

    match cli.command {
        Commands::Node(command) => run_node_inner(command),
    }
}

fn run_node_inner(cli: Box<NodeCommand>) -> Result<(), Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let config = create_configuration(cli);
    // TODO: handle the result
    runtime.block_on(dkg_node_service::build_node(config))
}

fn create_configuration(cli: Box<NodeCommand>) -> Config {
    let private_key_path = cli.data_dir.private_key.expect("Private key path must be provided");
    let db_path = cli.data_dir.db_path.expect("DB path must be provided");
    let skde_path = cli.data_dir.skde_params;
    let chain_type = cli.dkg.chain_type.try_into().expect("Invalid chain type");
    Config::new(
        cli.rpc.external_rpc_url(),
        cli.rpc.internal_rpc_url(),
        cli.rpc.cluster_rpc_url(),
        cli.rpc.leader_rpc_url,
        cli.dkg.role,
        cli.dkg.trusted_address,
        chain_type,
        cli.dkg.session_cycle,
        private_key_path,
        db_path,
        skde_path,
    )
}