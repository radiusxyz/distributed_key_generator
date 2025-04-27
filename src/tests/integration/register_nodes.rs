// use skde::key_generation::generate_partial_key;
use radius_sdk::json_rpc::client::{Id, RpcClient};
use tokio::time::{sleep, Duration};

use crate::{
    rpc::cluster::GetKeyGeneratorList,
    tests::utils::{
        cleanup_existing_processes, init_test_environment, register_nodes,
        register_nodes_with_duplicate_addresses, start_node, verify_mutual_registration,
    },
    Role,
};

#[tokio::test]
async fn test_register_nodes() {
    // Initialize test environment
    init_test_environment("submit partial key and ack test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    let (_leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

    // Solver node needs to communicate with Leader - index is set to 3 to use different ports
    let (_solver_process, _solver_ports, _solver_config) =
        start_node(Role::Solver, 2, &mut temp_dirs).await;

    let (_committee_process, committee_ports, committee_config) =
        start_node(Role::Committee, 3, &mut temp_dirs).await;

    // 2. Register nodes with each other
    register_nodes(
        &leader_ports,
        &leader_config,
        &committee_ports,
        &committee_config,
    )
    .await;

    // 3. Verify both nodes are registered with each other
    let (leader_found, committee_found) =
        verify_mutual_registration(&leader_ports, &committee_ports).await;

    assert!(
        leader_found,
        "Leader node not found in committee's key generator list"
    );
    assert!(
        committee_found,
        "Committee node not found in leader's key generator list"
    );

    // 4. Register nodes with duplicate addresses
    register_nodes_with_duplicate_addresses(
        &leader_ports,
        &committee_config,
        &committee_ports,
        &leader_config,
    )
    .await;

    // 5. Check if KeyGeneratorList still has 2 entries after duplicate registration attempt
    sleep(Duration::from_secs(2)).await;

    // Check Leader node's KeyGeneratorList
    let rpc_client = RpcClient::new().unwrap();
    let leader_cluster_url = format!("http://127.0.0.1:{}", leader_ports.cluster);
    let response = rpc_client
        .request::<_, serde_json::Value>(
            &leader_cluster_url,
            "get_key_generator_list",
            &GetKeyGeneratorList,
            Id::Null,
        )
        .await
        .unwrap();

    let leader_key_generator_count = response["key_generator_rpc_url_list"]
        .as_array()
        .unwrap()
        .len();

    assert_eq!(
        leader_key_generator_count,
        2,
        "Leader's key generator list should have exactly 2 entries after attempted duplicate registration"
    );

    // Check Committee node's KeyGeneratorList
    let committee_cluster_url = format!("http://127.0.0.1:{}", committee_ports.cluster);
    let response = rpc_client
        .request::<_, serde_json::Value>(
            &committee_cluster_url,
            "get_key_generator_list",
            &GetKeyGeneratorList,
            Id::Null,
        )
        .await
        .unwrap();

    let committee_key_generator_count = response["key_generator_rpc_url_list"]
        .as_array()
        .unwrap()
        .len();

    assert_eq!(
        committee_key_generator_count,
        2,
        "Committee's key generator list should have exactly 2 entries after attempted duplicate registration"
    );

    // 6. Cleanup processes
    cleanup_existing_processes();
}
