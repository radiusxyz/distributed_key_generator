use std::path::Path;

use radius_sdk::{json_rpc::server::RpcServer, kvstore::KvStoreBuilder};
use skde::delay_encryption::SkdeParams;
use tempfile::TempDir;
use tokio::task::JoinHandle;

use crate::{
    config::{Config, ConfigOption, Role},
    error::Error,
    state::AppState,
    types::*,
};

/// Creates a temporary configuration for test nodes
pub fn create_temp_config(
    role: Role,
    cluster_port: u16,
    external_port: u16,
    internal_port: u16,
) -> (Config, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut config_option = ConfigOption {
        path: Some(temp_dir.path().to_path_buf()),
        external_rpc_url: Some(format!("127.0.0.1:{}", external_port)),
        internal_rpc_url: Some(format!("127.0.0.1:{}", internal_port)),
        cluster_rpc_url: Some(format!("127.0.0.1:{}", cluster_port)),
        leader_cluster_rpc_url: None,
        role: Some(role.to_string()),
        radius_foundation_address: None,
        chain_type: None,
        partial_key_generation_cycle: None,
        partial_key_aggregation_cycle: None,
    };

    (
        Config::load(&mut config_option).expect("Failed to load config"),
        temp_dir,
    )
}

/// Initialize the key-value store database for testing
pub fn init_test_database(db_path: &Path) {
    KvStoreBuilder::default()
        .set_default_lock_timeout(5000)
        .set_txn_lock_timeout(5000)
        .build(db_path)
        .expect("Failed to build database")
        .init();

    // Initialize key generator list and key ID in database
    KeyGeneratorList::initialize().expect("Failed to initialize key generator list");
    KeyId::initialize().expect("Failed to initialize key ID");
}

/// Runs a node with the specified configuration for testing
pub async fn run_node(config: Config, skde_params: SkdeParams) -> Vec<JoinHandle<()>> {
    let db_dir = tempfile::Builder::new()
        .prefix("dkg-test-db")
        .tempdir()
        .expect("Failed to create database directory");

    // Initialize the database
    init_test_database(db_dir.path());

    // Initialize the application state
    let app_state = AppState::new(config.clone(), skde_params.clone());

    // Initialize all RPC servers
    let mut handles = Vec::new();

    // Initialize the internal RPC server
    let internal_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<crate::rpc::internal::AddKeyGenerator>()
        .expect("Failed to register internal RPC method")
        .init(app_state.config().internal_rpc_url().to_string())
        .await
        .expect("Failed to initialize internal RPC server");

    handles.push(tokio::spawn(async move {
        internal_rpc_server.stopped().await;
    }));

    // Initialize the cluster RPC server
    let cluster_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<crate::rpc::cluster::GetKeyGeneratorList>()
        .expect("Failed to register GetKeyGeneratorList RPC method")
        .register_rpc_method::<crate::rpc::cluster::SyncKeyGenerator>()
        .expect("Failed to register SyncKeyGenerator RPC method")
        .register_rpc_method::<crate::rpc::cluster::SyncAggregatedKey>()
        .expect("Failed to register SyncAggregatedKey RPC method")
        .register_rpc_method::<crate::rpc::cluster::SyncPartialKey>()
        .expect("Failed to register SyncPartialKey RPC method")
        .register_rpc_method::<crate::rpc::cluster::RunGeneratePartialKey>()
        .expect("Failed to register RunGeneratePartialKey RPC method")
        .init(app_state.config().cluster_rpc_url().to_string())
        .await
        .expect("Failed to initialize cluster RPC server");

    handles.push(tokio::spawn(async move {
        cluster_rpc_server.stopped().await;
    }));

    // Initialize the external RPC server
    let external_rpc_server = RpcServer::new(app_state.clone())
        .register_rpc_method::<crate::rpc::external::GetEncryptionKey>()
        .expect("Failed to register GetEncryptionKey RPC method")
        .register_rpc_method::<crate::rpc::external::GetDecryptionKey>()
        .expect("Failed to register GetDecryptionKey RPC method")
        .register_rpc_method::<crate::rpc::external::GetLatestEncryptionKey>()
        .expect("Failed to register GetLatestEncryptionKey RPC method")
        .register_rpc_method::<crate::rpc::external::GetLatestKeyId>()
        .expect("Failed to register GetLatestKeyId RPC method")
        .register_rpc_method::<crate::rpc::external::GetSkdeParams>()
        .expect("Failed to register GetSkdeParams RPC method")
        .init(app_state.config().external_rpc_url().to_string())
        .await
        .expect("Failed to initialize external RPC server");

    handles.push(tokio::spawn(async move {
        external_rpc_server.stopped().await;
    }));

    handles
}
