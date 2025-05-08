use tokio::time::{sleep, Duration};

use crate::{
    tests::utils::{cleanup_existing_processes, init_test_environment, register_nodes, start_node},
    Role,
};

#[tokio::test]
async fn test_run_verifier_logic() {
    // Initialize test environment
    init_test_environment("test_run_verification_logic");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    let (_leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

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

    // 3. Run verifier logic, leader external rpc url is 7201
    // let leader_external_rpc_url = format!("http://127.0.0.1:{}", leader_ports.external);
    // for i in 0..10 {
    //     let session_id = SessionId::new(i);
    //     let request = GetDecryptionKey { session_id };
    //     let response = rpc_client.request(leader_external_rpc_url, request).await;
    //     println!("response: {:?}", response);
    //     sleep(Duration::from_secs(5)).await;
    // }
    // sleep(Duration::from_secs(5)).await;

    // 6. Cleanup processes
    cleanup_existing_processes();
}
