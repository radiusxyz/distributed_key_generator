use radius_sdk::json_rpc::client::{Id, RpcClient};
use tokio::time::{sleep, Duration};
use tracing::info;

use crate::{
    rpc::common::{GetSkdeParams, GetSkdeParamsResponse},
    tests::utils::{
        cleanup_existing_processes, get_decryption_key, get_encryption_key,
        get_finalized_partial_keys, get_latest_session_id, init_test_environment, register_nodes,
        start_node, TEST_SESSION_CYCLE_MS,
    },
    PartialKeySubmission, Role,
};

// Helper function to compare PartialKeySubmission vectors
fn compare_partial_key_submissions(
    submissions1: &Vec<PartialKeySubmission>,
    submissions2: &Vec<PartialKeySubmission>,
) -> bool {
    // Different lengths means different lists
    if submissions1.len() != submissions2.len() {
        info!(
            "Partial key list lengths differ: {} vs {}",
            submissions1.len(),
            submissions2.len()
        );
        return false;
    }

    // TODO: Should fix it. Just test for ordering of partial keys
    let mut sorted_submissions1 = submissions1.clone();
    sorted_submissions1.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    let mut sorted_submissions2 = submissions2.clone();
    sorted_submissions2.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    info!(
        "Sorted submissions1: {:?}",
        sorted_submissions1
            .iter()
            .map(|s| s.payload.submit_timestamp)
            .collect::<Vec<_>>()
    );
    info!(
        "Sorted submissions2: {:?}",
        sorted_submissions2
            .iter()
            .map(|s| s.payload.submit_timestamp)
            .collect::<Vec<_>>()
    );

    // Compare important fields of each element
    for (i, (sub1, sub2)) in sorted_submissions1
        .iter()
        .zip(sorted_submissions2.iter())
        .enumerate()
    {
        // Compare sender addresses
        if sub1.payload.sender != sub2.payload.sender {
            info!(
                "Sender at index {} differs: {:?} vs {:?}",
                i, sub1.payload.sender, sub2.payload.sender
            );
            return false;
        }

        // Compare session IDs
        if sub1.payload.session_id != sub2.payload.session_id {
            info!(
                "Session ID at index {} differs: {:?} vs {:?}",
                i, sub1.payload.session_id, sub2.payload.session_id
            );
            return false;
        }

        // Compare partial keys (u, v, y, w values)
        let pk1 = &sub1.payload.partial_key;
        let pk2 = &sub2.payload.partial_key;
        if pk1.u != pk2.u || pk1.v != pk2.v || pk1.y != pk2.y || pk1.w != pk2.w {
            panic!("Partial key at index {} differs", i);
        }
    }

    true
}

// Verify encryption and decryption keys
async fn verify_key_pair(
    leader_url: &str,
    session_id: u64,
    skde_params: &skde::delay_encryption::SkdeParams,
    cycle_num: usize,
) -> bool {
    info!("Verifying key pair for session {}", session_id);

    // Get encryption key
    let encryption_key = match get_encryption_key(leader_url, session_id).await {
        Ok(key) => key,
        Err(e) => panic!("Failed to get encryption key: {:?}", e),
    };

    // Get decryption key
    let decryption_key = match get_decryption_key(leader_url, session_id).await {
        Ok(key) => key,
        Err(e) => panic!("Failed to get decryption key: {:?}", e),
    };

    // Verify key pair
    let prefix = format!("[Test Verifier {}]", cycle_num);
    match crate::utils::key::verify_encryption_decryption_key_pair(
        skde_params,
        &encryption_key,
        &decryption_key,
        &prefix,
    ) {
        Ok(_) => {
            info!("Cycle {}/3: Key pair verification successful ✅", cycle_num);
            true
        }
        Err(e) => {
            info!(
                "Cycle {}/3: Key pair verification failed ❌ - {:?}",
                cycle_num, e
            );
            false
        }
    }
}

// Compare partial key submissions between nodes
async fn compare_node_partial_keys(
    leader_url: &str,
    committee_url: &str,
    session_id: u64,
    cycle_num: usize,
) {
    info!("Comparing partial keys for session {}", session_id);

    // Get partial key submissions from leader
    let submissions_from_leader = match get_finalized_partial_keys(leader_url, session_id).await {
        Ok(submissions) => submissions,
        Err(e) => panic!("Failed to get partial keys from leader: {:?}", e),
    };

    // Get partial key submissions from committee
    let submissions_from_committee =
        match get_finalized_partial_keys(committee_url, session_id).await {
            Ok(submissions) => submissions,
            Err(e) => panic!("Failed to get partial keys from committee: {:?}", e),
        };

    // Compare submissions
    if compare_partial_key_submissions(&submissions_from_leader, &submissions_from_committee) {
        info!("Cycle {}/3: Partial key lists match ✅", cycle_num);
    } else {
        panic!("Cycle {}/3: Partial key lists do not match ❌", cycle_num);
    }
}

#[tokio::test]
async fn test_run_verifier_logic() {
    // Initialize test environment
    init_test_environment("test_run_verification_logic");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, authority_config) =
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

    // 3. Get SKDE params from leader
    let rpc_client = RpcClient::new().unwrap();
    let response: GetSkdeParamsResponse = rpc_client
        .request(
            &authority_config.leader_cluster_rpc_url().clone().unwrap(),
            "get_skde_params",
            &GetSkdeParams,
            Id::Null,
        )
        .await
        .unwrap();
    let skde_params = response.into_skde_params();

    // 4. Set up URLs for testing
    let leader_external_rpc_url = format!("http://127.0.0.1:{}", leader_ports.external);
    let committee_external_rpc_url = format!("http://127.0.0.1:{}", committee_ports.external);

    // Wait for 2 session cycles to ensure key generation has started
    info!("Waiting for key generation to start (2 session cycles)");
    sleep(Duration::from_millis(2000)).await;

    // 5. Run verification cycles
    for i in 0..3 {
        info!("Starting verification cycle {}/3", i + 1);

        // Get latest session ID
        let session_id = match get_latest_session_id(&leader_external_rpc_url).await {
            Ok(id) => id,
            Err(e) => panic!("Failed to get session ID: {:?}", e),
        };

        info!("Current session ID: {}", session_id);

        // Use previous session for verification (current might not be complete)
        let prev_session_id = session_id - 1;

        // Verify key pair
        verify_key_pair(
            &leader_external_rpc_url,
            prev_session_id,
            &skde_params,
            i + 1,
        )
        .await;

        // Compare partial key submissions between nodes
        compare_node_partial_keys(
            &leader_external_rpc_url,
            &committee_external_rpc_url,
            prev_session_id,
            i + 1,
        )
        .await;

        sleep(Duration::from_millis(TEST_SESSION_CYCLE_MS as u64)).await;
    }

    // 6. Cleanup processes
    cleanup_existing_processes();
    info!("Test completed successfully ✅");
}
