use crate::{
    tests::utils::{
        cleanup_existing_processes, init_test_environment, register_nodes, start_node,
        verify_mutual_registration,
    },
    Role,
};

#[tokio::test]
async fn test_regsiter_nodes() {
    // Initialize test environment
    init_test_environment("node registration test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    let (_leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

    let (_committee_process, committee_ports, committee_config) =
        start_node(Role::Committee, 3, &mut temp_dirs).await;

    // 2. Verify nodes are not registered to each other yet
    let (leader_found, committee_found) =
        verify_mutual_registration(&leader_ports, &committee_ports).await;

    assert!(!leader_found, "should fail to find committee node");
    assert!(!committee_found, "should fail to find leader node");

    // 3-4. Register nodes with each other
    register_nodes(
        &leader_ports,
        &leader_config,
        &committee_ports,
        &committee_config,
    )
    .await;

    // Verify nodes are registered to each other
    let (leader_found, committee_found) =
        verify_mutual_registration(&leader_ports, &committee_ports).await;

    assert!(
        leader_found,
        "Committee node not found in leader's key generator list"
    );
    assert!(
        committee_found,
        "Leader node not found in committee's key generator list"
    );

    // 5. Cleanup processes
    cleanup_existing_processes();
}
