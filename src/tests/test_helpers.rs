use std::path::Path;

use radius_sdk::{json_rpc::server::RpcServer, kvstore::KvStoreBuilder};
use skde::delay_encryption::SkdeParams;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tracing::info;

use crate::{
    config::{Config, ConfigOption, Role},
    rpc::external::GetEncryptionKey,
    state::AppState,
    types::*,
};

const TEST_PRIVATE_KEYS: [&str; 10] = [
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80", // 0
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d", // 1
    "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a", // 2
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6", // 3
    "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a", // 4
    "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba", // 5
    "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e", // 6
    "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356", // 7
    "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97", // 8
    "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6", // 9
];

/// according to the role, return the private key index
fn get_private_key_index(role: &Role) -> usize {
    match role {
        Role::Leader => 0,
        Role::Committee => 1,
        Role::Solver => 2,
        Role::Verifier => 3,
    }
}

/// Creates a temporary configuration for test nodes
pub fn create_temp_config(
    role: Role,
    cluster_port: u16,
    external_port: u16,
    internal_port: u16,
) -> (Config, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let key_index = get_private_key_index(&role);
    let private_key = TEST_PRIVATE_KEYS[key_index];

    let signing_key_path = temp_dir.path().join("signing_key");
    std::fs::write(&signing_key_path, private_key).expect("Failed to write signing key");

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
    SessionId::initialize().expect("Failed to initialize key ID");
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

    // Get the node address and create a shortened version for logging
    let address = app_state.config().address();
    let address_str = address.as_hex_string();
    let short_address = if address_str.len() >= 6 {
        format!("[{}]", &address_str[..6])
    } else {
        format!("[{}]", address_str)
    };

    // Log the node start with the shortened address
    info!(
        "Starting node {} with role: {}",
        short_address,
        app_state.role()
    );

    // Initialize all RPC servers
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    if app_state.is_solver() {
        let external_rpc_server = RpcServer::new(app_state.clone())
            .register_rpc_method::<GetEncryptionKey>()
            .expect("Failed to register GetEncryptionKey RPC method")
            .init(app_state.config().external_rpc_url().to_string())
            .await
            .expect("Failed to initialize external RPC server");

        handles.push(tokio::spawn(async move {
            external_rpc_server.stopped().await;
        }));
    } else {
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
            .register_rpc_method::<crate::rpc::cluster::SubmitPartialKey>()
            .expect("Failed to register SubmitPartialKey RPC method")
            .register_rpc_method::<crate::rpc::cluster::SubmitPartialKeyAck>()
            .expect("Failed to register SubmitPartialKeyAck RPC method")
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
    }

    handles
}
