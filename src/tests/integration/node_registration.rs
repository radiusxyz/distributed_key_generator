use radius_sdk::json_rpc::client::{Id, RpcClient};
use tokio::time::Duration;

use crate::{
    config::Role,
    error::Error,
    rpc::cluster::GetKeyGeneratorList,
    tests::{test_helpers, utils},
};

/// Test to verify that two nodes can connect and a follower node can register with a leader

#[tokio::test]
async fn test_two_nodes_key_generator_registration() {
    // Setup test logging
    utils::setup_test_logging();

    // Get default test port configuration
    let ports = utils::TestPortConfig::default();

    // Create SKDE params for nodes
    let skde_params_leader = utils::create_skde_params();
    let skde_params_follower = utils::create_skde_params();

    // Create leader node configuration
    let (leader_config, _leader_temp_dir) = test_helpers::create_temp_config(
        Role::Leader,
        ports.leader.cluster,
        ports.leader.external,
        ports.leader.internal,
    );

    // Create follower node configuration
    let (follower_config, _follower_temp_dir) = test_helpers::create_temp_config(
        Role::Committee,
        ports.follower.cluster,
        ports.follower.external,
        ports.follower.internal,
    );

    // Run nodes as async tasks
    let _leader_handles = test_helpers::run_node(leader_config, skde_params_leader).await;
    let _follower_handles = test_helpers::run_node(follower_config, skde_params_follower).await;

    // Wait for the servers to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Create test address for the follower
    let follower_address = utils::create_test_address("0x1234567890123456789012345678901234567890");

    // Create RPC URLs for the follower
    let cluster_rpc_url = format!("http://127.0.0.1:{}", ports.follower.cluster);
    let external_rpc_url = format!("http://127.0.0.1:{}", ports.follower.external);

    // Create JSON-RPC client
    let rpc_client = RpcClient::new().unwrap();

    // Create parameters for add_key_generator request
    let add_keygen_params = serde_json::json!({
        "message": {
            "address": follower_address.as_hex_string(),
            "cluster_rpc_url": cluster_rpc_url,
            "external_rpc_url": external_rpc_url
        }
    });

    // Register follower with leader
    rpc_client
        .request::<_, ()>(
            format!("http://127.0.0.1:{}", ports.leader.internal),
            "add_key_generator",
            &add_keygen_params,
            Id::Number(1),
        )
        .await
        .unwrap();

    // Verify the follower is registered by querying the key generator list
    let response: serde_json::Value = rpc_client
        .request(
            format!("http://127.0.0.1:{}", ports.leader.cluster),
            "get_key_generator_list",
            &GetKeyGeneratorList,
            Id::Number(2),
        )
        .await
        .unwrap();

    // Check if the follower is registered
    let key_generator_list = response["key_generator_rpc_url_list"]
        .as_array()
        .expect("Invalid response format");

    let followers_found = key_generator_list
        .iter()
        .any(|kg| kg["address"].as_str().unwrap_or("") == follower_address.as_hex_string());

    assert!(followers_found, "Follower not found in key generator list");
}
