use std::{
    path::PathBuf,
    process::{Child, Command},
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bincode::serialize as serialize_to_bincode;
use radius_sdk::{
    json_rpc::client::{Id, RpcClient},
    signature::{Address, ChainType},
};
use skde::{
    delay_encryption::SkdeParams,
    key_generation::{generate_partial_key, prove_partial_key_validity},
    BigUint,
};
use tempfile::TempDir;
use tracing::{info, Level};
use tracing_subscriber::fmt;

use crate::{
    config::Role,
    rpc::{
        cluster::{GetKeyGeneratorList, GetSkdeParams, GetSkdeParamsResponse, PartialKeyPayload},
        common::create_signature,
    },
    types::{Config, ConfigOption},
    SessionId,
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

// Constants for SKDE parameters
const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
const GENERATOR: &str = "4";
const TIME_PARAM_T: u32 = 2;
const MAX_KEY_GENERATOR_NUMBER: u32 = 2;

/// Creates SKDE parameters for testing purposes
pub fn create_skde_params() -> SkdeParams {
    let n = BigUint::from_str(MOD_N).expect("Invalid MOD_N");
    let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
    let max_key_generator_number = BigUint::from(MAX_KEY_GENERATOR_NUMBER);
    let t = 2_u32.pow(TIME_PARAM_T);
    let mut h = g.clone();
    (0..t).for_each(|_| {
        h = (&h * &h) % n.clone();
    });

    SkdeParams {
        t,
        n: n.to_str_radix(10),
        g: g.to_str_radix(10),
        h: h.to_str_radix(10),
        max_sequencer_number: max_key_generator_number.to_str_radix(10),
    }
}

/// Creates a test Ethereum address
pub fn create_test_address(address_str: &str) -> Address {
    Address::from_str(ChainType::Ethereum, address_str).unwrap()
}

/// Port definitions for testing
#[derive(Debug)]
pub struct TestPorts {
    pub cluster: u16,
    pub external: u16,
    pub internal: u16,
}

/// Setup tracing for tests
pub fn setup_test_logging() {
    use tracing::Level;
    use tracing_subscriber::fmt;

    let _ = fmt()
        .with_max_level(Level::INFO)
        .with_test_writer()
        .try_init();
}

/// Find binary path (debug or release)
pub fn find_binary_path() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    // Check release binary
    let release_path = current_dir
        .join("target")
        .join("release")
        .join("key-generator");
    if release_path.exists() {
        return release_path;
    }

    // Check debug binary
    let debug_path = current_dir
        .join("target")
        .join("debug")
        .join("key-generator");
    if debug_path.exists() {
        return debug_path;
    }

    panic!("key-generator binary not found. Build it first: cargo build");
}

/// Create Config object from Config.toml file
pub fn create_config_from_dir(temp_path: &PathBuf) -> Config {
    // Create ConfigOption with path
    let mut config_option = ConfigOption {
        path: Some(temp_path.clone()),
        authority_rpc_url: None,
        external_rpc_url: None,
        internal_rpc_url: None,
        cluster_rpc_url: None,
        leader_cluster_rpc_url: None,
        role: None,
        radius_foundation_address: None,
        chain_type: None,
        partial_key_generation_cycle: None,
        partial_key_aggregation_cycle: None,
    };

    // Load Config (automatically reads from Config.toml)
    Config::load(&mut config_option).expect("Failed to load Config")
}

/// Start a test node with specified role
pub fn spawn_node_process(
    role: Role,
    index: usize,
    temp_dirs: &mut Vec<TempDir>,
) -> (Child, TestPorts, Config) {
    assert!(index < 10, "index should be less than 10");

    // Set unique ports for each node
    let internal_port: u16 = (7100 + index).try_into().unwrap();
    let external_port: u16 = (7200 + index).try_into().unwrap();
    let cluster_port: u16 = (7300 + index).try_into().unwrap();

    // Authority 노드는 프로젝트 루트의 /data 디렉토리 사용, 다른 노드는 임시 디렉토리 사용
    let (temp_path, temp_dir) = if role == Role::Authority {
        // 프로젝트 루트 디렉토리 찾기
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let project_root = current_dir.clone(); // 프로젝트 루트로 가정

        // data 디렉토리 생성
        let data_path = project_root.join("data/authority");
        std::fs::create_dir_all(&data_path).expect("Failed to create data directory");

        info!(
            "Created {} node directory at project root: {:?}",
            role, data_path
        );
        (data_path, None) // 임시 디렉토리가 아니므로 None 반환
    } else {
        // 기존과 같이 임시 디렉토리 생성
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path().to_path_buf();

        info!("Created {} node directory: {:?}", role, temp_path);
        (temp_path, Some(temp_dir))
    };

    let authority_rpc_url = format!("authority_rpc_url = \"http://127.0.0.1:6000\"");

    // Create Config.toml file
    let config_path = temp_path.join("Config.toml");
    // Non-leader nodes need leader URL
    let leader_url = if role != Role::Leader {
        format!("leader_cluster_rpc_url = \"http://127.0.0.1:7300\"")
    } else {
        "".to_string()
    };

    let config_content = format!(
        r#"# NODE CONFIG: {} Node (Node {})
external_rpc_url = "http://127.0.0.1:{}"
internal_rpc_url = "http://127.0.0.1:{}"
cluster_rpc_url = "http://127.0.0.1:{}"
role = "{}"
radius_foundation_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
chain_type = "ethereum"
partial_key_generation_cycle = 5
partial_key_aggregation_cycle = 4
{}
{}
"#,
        role,
        index,
        external_port,
        internal_port,
        cluster_port,
        role.to_string().to_lowercase(),
        leader_url,
        authority_rpc_url
    );

    std::fs::write(&config_path, config_content).expect("Failed to write Config.toml");
    info!("Completed writing config file for {} node", role);

    // Write signing key file (different keys for different nodes)
    let key_index = if role == Role::Leader { 0 } else { 1 };
    let signing_key_path = temp_path.join("signing_key");
    std::fs::write(&signing_key_path, TEST_PRIVATE_KEYS[key_index])
        .expect("Failed to write signing key");

    // Create independent database directory
    let db_path = temp_path.join("database");
    std::fs::create_dir_all(&db_path).expect("Failed to create database directory");

    // Create Config object
    let config = create_config_from_dir(&temp_path);
    info!("Loaded configuration for {} node", role);

    // Find binary path
    let binary_path = find_binary_path();
    info!("Binary path: {:?}", binary_path);

    // Run process
    info!(
        "Starting {} node (ext: {}, int: {}, cluster: {})",
        role, external_port, internal_port, cluster_port
    );

    let child = Command::new(binary_path)
        .current_dir(&temp_path)
        .arg("start")
        .arg("--path")
        .arg(".")
        .spawn()
        .expect("Failed to start node process");

    // Save temp directory (will be cleaned up after test)
    if let Some(dir) = temp_dir {
        temp_dirs.push(dir);
    }

    // Wait for node to start
    info!(
        "Waiting for {} node to start... (PID: {})",
        role,
        child.id()
    );
    sleep(Duration::from_secs(2));

    (
        child,
        TestPorts {
            cluster: cluster_port,
            external: external_port,
            internal: internal_port,
        },
        config,
    )
}

/// Register node with target node
pub async fn register_node(target_node_ports: &TestPorts, node_config: &Config) {
    // Create node address
    let node_address = node_config.address();

    // Create RPC URLs for the node
    let cluster_rpc_url = format!(
        "http://127.0.0.1:{}",
        node_config
            .cluster_rpc_url()
            .split(':')
            .last()
            .unwrap_or("")
    );
    let external_rpc_url = format!(
        "http://127.0.0.1:{}",
        node_config
            .external_rpc_url()
            .split(':')
            .last()
            .unwrap_or("")
    );

    // Create parameters for add_key_generator
    let add_key_generator = serde_json::json!({
        "message": {
            "address": node_address.as_hex_string(),
            "cluster_rpc_url": cluster_rpc_url,
            "external_rpc_url": external_rpc_url
        }
    });

    println!(
        "add_key_generator: {:?}, target_node_ports.internal: {:?}",
        add_key_generator, target_node_ports.internal
    );

    // Register node
    let rpc_client = RpcClient::new().unwrap();
    rpc_client
        .request::<_, ()>(
            format!("http://127.0.0.1:{}", target_node_ports.internal),
            "add_key_generator",
            &add_key_generator,
            Id::Number(1),
        )
        .await
        .unwrap();
}

/// Verify if base node is registered with target node
pub async fn verify_node_registration(target_port: u16, base_port: u16) -> bool {
    let rpc_client = RpcClient::new().unwrap();
    let cluster_url = format!("http://127.0.0.1:{}", base_port);

    info!(
        "Checking node registration: {} -> {}",
        cluster_url, target_port
    );

    // Call GetKeyGeneratorList RPC
    let response: serde_json::Value = match rpc_client
        .request(
            &cluster_url,
            "get_key_generator_list",
            &GetKeyGeneratorList,
            Id::Null,
        )
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            info!("RPC request failed: {:?}", e);
            serde_json::json!({ "key_generator_rpc_url_list": [] })
        }
    };

    // Check if target node URL is in the key generator list
    let empty_vec = Vec::new();
    let key_generator_list = response["key_generator_rpc_url_list"]
        .as_array()
        .unwrap_or(&empty_vec);

    let target_url = format!("http://127.0.0.1:{}", target_port);

    info!(
        "Received key generator list: {} items",
        key_generator_list.len()
    );

    for generator in key_generator_list {
        let cluster_url = generator["cluster_rpc_url"].as_str().unwrap_or("");
        info!("Checking generator: {}", cluster_url);
        if cluster_url == target_url {
            info!("Target node found: {}", target_url);
            return true;
        }
    }
    info!("Target node not found: {}", target_url);
    false
}

/// Generates a partial key and its proof using SKDE parameters
pub async fn generate_partial_key_with_proof(
    committee_config: &Config,
) -> (
    skde::key_generation::SecretValue,
    skde::key_generation::PartialKey,
    skde::key_generation::PartialKeyProof,
) {
    let rpc_client = RpcClient::new().unwrap();

    let skde_params: SkdeParams = {
        let response: GetSkdeParamsResponse = rpc_client
            .request(
                &committee_config.leader_cluster_rpc_url().clone().unwrap(),
                "get_skde_params",
                &GetSkdeParams,
                Id::Null,
            )
            .await
            .unwrap();
        response.into_skde_params()
    };

    // Generate partial key
    let (secret_value, partial_key) = generate_partial_key(&skde_params).unwrap();

    // Generate partial key validity proof
    let partial_key_proof = prove_partial_key_validity(&skde_params, &secret_value).unwrap();

    (secret_value, partial_key, partial_key_proof)
}

/// Generates the current timestamp
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Submits a partial key from a committee node to a leader node
pub async fn submit_partial_key_to_leader(
    committee_address: Address,
    leader_port: u16,
    partial_key: skde::key_generation::PartialKey,
    partial_key_proof: skde::key_generation::PartialKeyProof,
    session_id: SessionId,
) {
    info!("Submitting partial key from committee to leader");

    // Generate timestamp
    let timestamp = get_current_timestamp();

    // Create RPC client
    let rpc_client = RpcClient::new().unwrap();

    // Create partial key payload
    let payload = PartialKeyPayload {
        sender: committee_address.clone(),
        partial_key: partial_key.clone(),
        proof: partial_key_proof.clone(),
        submit_timestamp: timestamp,
        session_id,
    };

    // Generate signature
    let signature = create_signature(&serialize_to_bincode(&payload).unwrap());

    // Create JSON parameter
    let parameter = serde_json::json!({
        "signature": signature,
        "payload": {
            "sender": committee_address,
            "partial_key": partial_key,
            "proof": partial_key_proof,
            "submit_timestamp": timestamp,
            "session_id": session_id
        }
    });

    // Submit partial key to leader node
    let _response: () = rpc_client
        .request(
            format!("http://127.0.0.1:{}", leader_port),
            "submit_partial_key",
            &parameter,
            Id::Number(1),
        )
        .await
        .unwrap();
}

/// Initialize logging for tests
pub fn init_test_logging() {
    let subscriber = fmt().with_max_level(Level::INFO).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Clean up any existing key-generator processes
pub fn cleanup_existing_processes() {
    let _ = Command::new("pkill")
        .arg("-f")
        .arg("key-generator")
        .output();
}

/// Log test start message
pub fn log_test_start(test_name: &str) {
    info!("Starting distributed key generation {}", test_name);
}

/// Initialize test environment (logging and cleanup existing processes)
pub fn init_test_environment(test_name: &str) {
    cleanup_existing_processes();
    init_test_logging();
    log_test_start(test_name);
}

/// Start a single node with specified role and index
pub async fn start_node(
    role: Role,
    index: usize,
    temp_dirs: &mut Vec<TempDir>,
) -> (
    std::process::Child,  // node_process
    TestPorts,            // node_ports
    crate::types::Config, // node_config
) {
    // Start node
    let (node_process, node_ports, node_config) = spawn_node_process(role, index, temp_dirs);

    info!("Waiting for node initialization");
    // Wait for node initialization
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    (node_process, node_ports, node_config)
}

/// Register nodes with each other
pub async fn register_nodes(
    leader_ports: &TestPorts,
    leader_config: &crate::types::Config,
    committee_ports: &TestPorts,
    committee_config: &crate::types::Config,
) {
    // Register leader with committee node
    register_node(leader_ports, committee_config).await;

    // Register committee with leader node
    register_node(committee_ports, leader_config).await;
}

/// Verify mutual registration between nodes
pub async fn verify_mutual_registration(
    leader_ports: &TestPorts,
    committee_ports: &TestPorts,
) -> (bool, bool) {
    // Check if leader can find committee
    let leader_found =
        verify_node_registration(leader_ports.cluster, committee_ports.cluster).await;

    // Check if committee can find leader
    let committee_found =
        verify_node_registration(committee_ports.cluster, leader_ports.cluster).await;

    (leader_found, committee_found)
}

/// Clean up processes
pub fn cleanup_processes(
    leader_process: &mut std::process::Child,
    committee_process: &mut std::process::Child,
) {
    info!("Test complete, cleaning up processes");
    if let Err(e) = leader_process.kill() {
        info!("Failed to kill leader process: {:?}", e);
    }

    if let Err(e) = committee_process.kill() {
        info!("Failed to kill committee process: {:?}", e);
    }
}
