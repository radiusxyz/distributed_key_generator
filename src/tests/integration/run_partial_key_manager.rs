use tokio::time::{sleep, Duration};

use crate::{
    tests::utils::{
        cleanup_all_processes, init_test_environment, register_nodes, start_node,
        verify_mutual_registration,
    },
    Role,
};

#[tokio::test]
async fn test_integration_run_partial_key_manager() {
    // Initialize test environment
    init_test_environment("submit partial key and ack test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (mut authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 9, &mut temp_dirs).await;

    let (mut leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 0, &mut temp_dirs).await;

    let (mut committee_process, committee_ports, committee_config) =
        start_node(Role::Committee, 1, &mut temp_dirs).await;

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

    sleep(Duration::from_secs(20)).await;

    // 6. Cleanup processes
    let mut processes = vec![
        &mut authority_process,
        &mut leader_process,
        &mut committee_process,
    ];
    cleanup_all_processes(&mut processes);
}
