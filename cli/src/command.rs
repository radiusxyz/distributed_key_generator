use crate::{Cli, Commands, node::NodeCommand, trusted_setup::{Method, TrustedSetupCommand, run_skde_inner}};
use dkg_node_primitives::NodeConfig;
use dkg_primitives::RuntimeResult;
use std::{path::PathBuf, time::Duration};

pub fn run() -> RuntimeResult<()> {
    let cli = Cli::init();

    match cli.command {
        Commands::Node(command) => run_node_inner(command),
        Commands::TrustedSetup(command) => run_trusted_setup_inner(command),
    }
}

fn run_node_inner(cli: Box<NodeCommand>) -> RuntimeResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let config = create_configuration(cli);
    // TODO: handle the result
    runtime.block_on(dkg_node_service::run_node(config))
}

fn create_configuration(cli: Box<NodeCommand>) -> NodeConfig {
    let private_key_path = cli.data_dir.private_key.map_or(PathBuf::from(format!("./tmp/{}/private_key", cli.dkg.role)), |path| path.into());
    let db_path = cli.data_dir.db_path.map_or(PathBuf::from(format!("./tmp/{}/db", cli.dkg.role)), |path| path.into());
    let trusted_setup_path = cli.data_dir.trusted_setup.map_or(PathBuf::from(format!("./tmp/{}/trusted_setup", cli.dkg.role)), |path| path.into());
    let chain_type = cli.dkg.chain_type.try_into().expect("Invalid chain type");
    NodeConfig::new(
        cli.rpc.external_rpc_url(),
        cli.rpc.internal_rpc_url(),
        cli.rpc.cluster_rpc_url(),
        cli.dkg.role,
        cli.dkg.trusted_address,
        cli.dkg.auth_service_endpoint,
        chain_type,
        Duration::from_millis(cli.dkg.session_duration),
        private_key_path,
        db_path,
        Some(trusted_setup_path),
        cli.dkg.threshold,
        cli.dkg.round_look_ahead,
    )
}

fn run_trusted_setup_inner(cli: Box<TrustedSetupCommand>) -> RuntimeResult<()> {
    match cli.method {
        Method::Skde(args) => run_skde_inner(args),
    }
    Ok(())
}