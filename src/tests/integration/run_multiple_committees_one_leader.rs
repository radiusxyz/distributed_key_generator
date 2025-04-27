use tokio::time::{sleep, Duration};

use crate::{
    tests::utils::{
        cleanup_existing_processes, init_test_environment, register_nodes, start_node,
        verify_mutual_registration,
    },
    Role,
};

#[tokio::test]
async fn test_run_multiple_committees_one_leader() {
    // Initialize test environment
    init_test_environment("multiple committees with staggered peering test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority node
    let (_authority_process, _authority_ports, _authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    // 2. Start leader node
    let (_leader_process, leader_ports, leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

    // 3. Start solver node
    let (_solver_process, _solver_ports, _solver_config) =
        start_node(Role::Solver, 2, &mut temp_dirs).await;

    // 4. Start committee nodes (8 nodes in total)
    let mut committee_details = Vec::new();

    // Create all committee nodes 3 ~ 8, total 6 nodes
    for i in 0..6 {
        let (_, ports, config) = start_node(Role::Committee, i + 3, &mut temp_dirs).await;
        committee_details.push((ports, config));
    }

    // 4. Register nodes in stages with time intervals

    // Stage 1: Register first 2 committee nodes
    println!("Registering first batch of committee nodes");
    for i in 0..2 {
        let (ports, config) = &committee_details[i];

        register_nodes(&leader_ports, &leader_config, ports, config).await;

        // Verify registration
        let (leader_found, committee_found) =
            verify_mutual_registration(&leader_ports, ports).await;

        assert!(
            leader_found,
            "Leader node not found in committee node {}'s key generator list",
            i + 1
        );
        assert!(
            committee_found,
            "Committee node {} not found in leader's key generator list",
            i + 1
        );
    }

    // Wait 3 seconds before next batch
    println!("Waiting 3 seconds before registering next batch...");
    sleep(Duration::from_secs(3)).await;

    // Stage 2: Register next 2 committee nodes
    println!("Registering second batch of committee nodes");
    for i in 2..4 {
        let (ports, config) = &committee_details[i];

        register_nodes(&leader_ports, &leader_config, ports, config).await;

        // Verify registration
        let (leader_found, committee_found) =
            verify_mutual_registration(&leader_ports, ports).await;

        assert!(
            leader_found,
            "Leader node not found in committee node {}'s key generator list",
            i + 1
        );
        assert!(
            committee_found,
            "Committee node {} not found in leader's key generator list",
            i + 1
        );
    }

    // Wait 3 seconds before final batch
    println!("Waiting 3 seconds before registering final batch...");
    sleep(Duration::from_secs(3)).await;

    // Stage 3: Register final 3 committee nodes
    println!("Registering final batch of committee nodes");
    for i in 4..6 {
        let (ports, config) = &committee_details[i];

        register_nodes(&leader_ports, &leader_config, ports, config).await;

        // Verify registration
        let (leader_found, committee_found) =
            verify_mutual_registration(&leader_ports, ports).await;

        assert!(
            leader_found,
            "Leader node not found in committee node {}'s key generator list",
            i + 1
        );
        assert!(
            committee_found,
            "Committee node {} not found in leader's key generator list",
            i + 1
        );
    }

    // Wait for some time to observe system behavior
    println!("All nodes registered. Waiting to observe system behavior...");
    sleep(Duration::from_secs(10)).await;

    // 5. Cleanup all processes
    println!("Test completed. Cleaning up processes...");
    cleanup_existing_processes();
}
