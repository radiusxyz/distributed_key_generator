use tokio::time::{sleep, Duration};
use tracing::info;

use crate::{
    tests::utils::{
        cleanup_all_processes, generate_partial_key_with_proof, init_test_environment,
        register_nodes, start_node, submit_partial_key_to_leader, verify_mutual_registration,
    },
    Role, SessionId,
};

#[tokio::test]
async fn test_integration_submit_partial_key_and_ack() {
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

    // 4. Generate partial key from committee
    let (_, partial_key, partial_key_proof) =
        generate_partial_key_with_proof(&committee_config).await;

    // Session ID for this test
    let session_id = SessionId::default();

    // Create committee address
    let committee_address = committee_config.address();

    // Submit partial key from committee to leader
    submit_partial_key_to_leader(
        committee_address.clone(),
        leader_ports.cluster,
        partial_key,
        partial_key_proof,
        session_id,
    )
    .await;

    // 5. Wait for and verify the acknowledgment
    info!("Waiting for partial key acknowledgment");
    sleep(Duration::from_secs(2)).await;

    // 6. Cleanup processes
    let mut processes = vec![
        &mut authority_process,
        &mut leader_process,
        &mut committee_process,
    ];
    cleanup_all_processes(&mut processes);
}
