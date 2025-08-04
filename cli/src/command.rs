use crate::{Cli, Commands, node::NodeCommand, trusted_setup::{Method, TrustedSetupCommand, run_skde_inner}, load_config, TomlConfig};
use dkg_node_primitives::{Role, NodeConfig, config::{DEFAULT_EXTERNAL_RPC_URL, DEFAULT_INTERNAL_RPC_URL, DEFAULT_CLUSTER_RPC_URL, DEFAULT_CHAIN_TYPE}};
use dkg_primitives::RuntimeResult;
use std::{path::PathBuf, str::FromStr};

pub fn run() -> RuntimeResult<()> {
    let cli = Cli::init();

    let maybe_config = load_config(cli.config)?;

    match cli.command {
        Commands::Node(command) => run_node_inner(command, maybe_config),
        Commands::TrustedSetup(command) => run_trusted_setup_inner(command),
    }
}

fn run_node_inner(cli: Box<NodeCommand>, maybe_config: Option<TomlConfig>) -> RuntimeResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let config = create_configuration(cli, maybe_config);
    // TODO: handle the result
    runtime.block_on(dkg_node_service::run_node(config))
}

fn create_configuration(cli: Box<NodeCommand>, maybe_config: Option<TomlConfig>) -> NodeConfig {
    // Data dir config
    // Role config
    let node_name = cli.node_name
        .or_else(|| maybe_config.as_ref().and_then(|config| config.node_name.clone()))
        .expect("Node name is required either in cli or config");
    let role = cli.dkg.role
        .or_else(|| maybe_config.as_ref().and_then(|config| config.role.clone().map(|role| Role::from_str(&role).unwrap())))
        .expect("Role is required either in cli or config");
    let private_key_path = cli.data_dir.private_key
        .or_else(|| maybe_config.as_ref().and_then(|config| config.private_key_path.clone()))
        .unwrap_or(PathBuf::from(format!("./tmp/{}/private_key", role)));
    let db_path = cli.data_dir.db_path
        .or_else(|| maybe_config.as_ref().and_then(|config| config.db_path.clone()))
        .unwrap_or(PathBuf::from(format!("./tmp/{}/db", role)));
    let trusted_setup_path = cli.data_dir.trusted_setup
        .or_else(|| maybe_config.as_ref().and_then(|config| config.trusted_setup_path.clone()))
        .unwrap_or(PathBuf::from(format!("./tmp/{}/trusted_setup", role)));

    let chain_type = cli.dkg.chain_type
        .or_else(|| maybe_config.as_ref().and_then(|config| config.chain_type.clone()))
        .unwrap_or(DEFAULT_CHAIN_TYPE.to_string());
    
    // Dkg config
    let external_rpc_url = cli.rpc.external_rpc_url
        .or_else(|| maybe_config.as_ref().and_then(|config| config.external_rpc.clone()))
        .unwrap_or_else(|| DEFAULT_EXTERNAL_RPC_URL.to_string());
    let internal_rpc_url = cli.rpc.internal_rpc_url
        .or_else(|| maybe_config.as_ref().and_then(|config| config.internal_rpc.clone()))
        .unwrap_or_else(|| DEFAULT_INTERNAL_RPC_URL.to_string());
    let cluster_rpc_url = cli.rpc.cluster_rpc_url
        .or_else(|| maybe_config.as_ref().and_then(|config| config.cluster_rpc.clone()))
        .unwrap_or_else(|| DEFAULT_CLUSTER_RPC_URL.to_string());

    let trusted_address = cli.dkg.trusted_address
        .or_else(|| maybe_config.as_ref().and_then(|config| config.trusted_address.clone()))
        .expect("Trusted address is required either in cli or config");

    let operator_service_url = cli.dkg.auth_service_url
        .or_else(|| maybe_config.as_ref().and_then(|config| config.auth_service_url.clone()))
        .expect("Auth service url is required either in cli or config");

    NodeConfig::new(
        cli.is_dev,
        Some(node_name),
        external_rpc_url,
        internal_rpc_url,
        cluster_rpc_url,
        role,
        trusted_address,
        operator_service_url,
        chain_type,
        private_key_path,
        db_path,
        Some(trusted_setup_path),
    )
}

fn run_trusted_setup_inner(cli: Box<TrustedSetupCommand>) -> RuntimeResult<()> {
    match cli.method {
        #[cfg(feature = "skde")]
        Method::Skde(args) => run_skde_inner(args),
    }
    Ok(())
}