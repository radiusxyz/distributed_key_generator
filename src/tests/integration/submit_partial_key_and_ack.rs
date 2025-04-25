// use skde::key_generation::generate_partial_key;
use tokio::time::{sleep, Duration};

use crate::{
    tests::utils::{
        cleanup_existing_processes, generate_partial_key_with_proof, init_test_environment,
        register_nodes, start_node, submit_partial_key_to_leader, verify_mutual_registration,
    },
    Role, SessionId,
};

// TODO: This test should be removed because test_integration_run_single_node_for_each_role.rs will cover this
#[tokio::test]
async fn test_submit_partial_key_and_ack() {
    // Initialize test environment
    init_test_environment("submit partial key and ack test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    let (_leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

    // Solver 노드는 Leader와 통신해야 함 - index는 3으로 설정하여 다른 포트 사용
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

    // // 4. Generate partial key from committee
    let (_, partial_key, _) = generate_partial_key_with_proof(&committee_config).await;

    // Session ID for this test
    let session_id = SessionId::default();

    // Create committee address
    let committee_address = committee_config.address();

    // Submit partial key from committee to leader
    submit_partial_key_to_leader(
        committee_address.clone(),
        leader_ports.cluster,
        partial_key,
        session_id,
    )
    .await;

    // 5. Wait for and verify the acknowledgment
    sleep(Duration::from_secs(2)).await;

    // 6. Cleanup processes
    cleanup_existing_processes();
}
