use radius_sdk::json_rpc::client::{Id, RpcClient};
use skde::{
    key_aggregation::{aggregate_key, AggregatedKey},
    key_generation::PartialKey,
};
use tokio::time::{sleep, Duration};
use tracing::info;

use crate::{
    rpc::common::{GetSkdeParams, GetSkdeParamsResponse},
    tests::utils::{
        cleanup_existing_processes, get_decryption_key, get_encryption_key,
        get_finalized_partial_keys, get_latest_session_id, init_test_environment,
        mock_get_randomness, register_nodes, start_node, TEST_SESSION_CYCLE_MS,
    },
    utils::{
        key::{derive_partial_key, select_random_partial_keys},
        signature::verify_signature,
    },
    PartialKeySubmission, Role,
};

// Helper function to compare PartialKeySubmission vectors
fn compare_partial_key_submissions(
    submissions1: &Vec<PartialKeySubmission>,
    submissions2: &Vec<PartialKeySubmission>,
) -> Vec<PartialKeySubmission> {
    // Different lengths means different lists
    if submissions1.len() != submissions2.len() {
        panic!(
            "Partial key list lengths differ: {} vs {}",
            submissions1.len(),
            submissions2.len()
        );
    }

    // Should fix it. Just test logic for fixed order of partial keys
    let mut sorted_submissions1 = submissions1.clone();
    sorted_submissions1.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    let mut sorted_submissions2 = submissions2.clone();
    sorted_submissions2.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));

    // Compare important fields of each element
    for (i, (sub1, sub2)) in sorted_submissions1
        .iter()
        .zip(sorted_submissions2.iter())
        .enumerate()
    {
        // Compare partial keys (u, v, y, w values)
        let pk1 = &sub1.payload.partial_key;
        let pk2 = &sub2.payload.partial_key;
        if pk1.u != pk2.u || pk1.v != pk2.v || pk1.y != pk2.y || pk1.w != pk2.w {
            panic!("Partial key at index {} differs", i);
        }
    }

    sorted_submissions2
}

async fn mock_aggregate_key(
    leader_url: &str,
    session_id: u64,
    skde_params: &skde::delay_encryption::SkdeParams,
    partial_key_list: &Vec<PartialKey>,
) -> AggregatedKey {
    let randomness = mock_get_randomness(leader_url, session_id).await;
    let mut selected_keys = select_random_partial_keys(partial_key_list, &randomness);
    let derived_key = derive_partial_key(&selected_keys, &skde_params);
    selected_keys.push(derived_key);
    aggregate_key(&skde_params, &selected_keys)
}

// Verify encryption and decryption keys
async fn verify_all_keys_in_session(
    leader_url: &str,
    session_id: u64,
    skde_params: &skde::delay_encryption::SkdeParams,
    finalized_partial_keys: Vec<PartialKeySubmission>,
) -> bool {
    info!("Verifying all keys in session {}", session_id);

    // Verify all signatures of partial keys
    for partial_key in finalized_partial_keys.iter() {
        let signable_message = partial_key.payload.clone();
        let signer = verify_signature(&partial_key.signature, &signable_message).unwrap();
        assert_eq!(signer, partial_key.payload.sender);
    }

    // Aggreagted partial keys
    let partial_key_list = finalized_partial_keys
        .iter()
        .map(|s| s.payload.partial_key.clone())
        .collect::<Vec<_>>();

    // Get encryption key
    let encryption_key = match get_encryption_key(leader_url, session_id).await {
        Ok(key) => key,
        Err(e) => panic!("Failed to get encryption key: {:?}", e),
    };

    // Verify aggregated key from partial keys
    let aggregated_key =
        mock_aggregate_key(leader_url, session_id, skde_params, &partial_key_list).await;
    assert_eq!(aggregated_key.u, encryption_key);

    // Get decryption key
    let decryption_key = match get_decryption_key(leader_url, session_id).await {
        Ok(key) => key,
        Err(e) => panic!("Failed to get decryption key: {:?}", e),
    };

    // Verify encryption and decryption key pair
    let prefix = format!("[Test Verifier]");
    match crate::utils::key::verify_encryption_decryption_key_pair(
        skde_params,
        &encryption_key,
        &decryption_key,
        &prefix,
    ) {
        Ok(_) => {
            info!("Key pair verification successful ✅");
            true
        }
        Err(e) => {
            panic!("Key pair verification failed ❌ - {:?}", e);
        }
    }
}

// Compare partial key submissions between nodes
async fn compare_node_partial_keys(
    leader_url: &str,
    committee_url: &str,
    session_id: u64,
) -> Vec<PartialKeySubmission> {
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
    compare_partial_key_submissions(&submissions_from_leader, &submissions_from_committee)
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

    // 4. Get external URLs for testing
    let leader_external_rpc_url = format!("http://127.0.0.1:{}", leader_ports.external);
    let committee_external_rpc_url = format!("http://127.0.0.1:{}", committee_ports.external);

    info!("Waiting for session_id = 2 at least");
    sleep(Duration::from_millis(2000)).await;

    // 5. Run verification cycles
    let iter_num = 3;
    for i in 0..iter_num {
        info!("Starting verification cycle {}/{}", i + 1, iter_num);

        // Get latest session ID
        let session_id = match get_latest_session_id(&leader_external_rpc_url).await {
            Ok(id) => id,
            Err(e) => panic!("Failed to get session ID: {:?}", e),
        };

        info!("Current session ID: {}", session_id);

        // Use previous session for verification (current might not be complete)
        let prev_session_id = session_id - 1;

        // Compare partial key submissions between nodes
        let finalized_partial_keys = compare_node_partial_keys(
            &leader_external_rpc_url,
            &committee_external_rpc_url,
            prev_session_id,
        )
        .await;

        // Verify key pair
        verify_all_keys_in_session(
            &leader_external_rpc_url,
            prev_session_id,
            &skde_params,
            finalized_partial_keys,
        )
        .await;

        // TODO: Add verifications
        // 1) Timestamp of Partial key, AggregatedKey, Decryption key
        // 2) Check if Partial key is valid(secret key, range proof)
        sleep(Duration::from_millis(TEST_SESSION_CYCLE_MS as u64)).await;
    }

    // 6. Cleanup processes
    cleanup_existing_processes();
    info!("Test completed successfully ✅");
}
