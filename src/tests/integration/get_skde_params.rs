use std::fs;

use radius_sdk::json_rpc::client::{Id, RpcClient};

use crate::{
    rpc::{
        authority::GetAuthorizedSkdeParams,
        cluster::{GetSkdeParams, GetSkdeParamsResponse},
    },
    tests::utils::{cleanup_existing_processes, init_test_environment, start_node},
    Role,
};

#[tokio::test]
async fn test_get_skde_params() {
    // Initialize test environment
    init_test_environment("node registration test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (_authority_process, _authority_ports, authority_config) =
        start_node(Role::Authority, 0, &mut temp_dirs).await;

    // Get authorized skde params from authority
    let rpc_client = RpcClient::new().unwrap();

    let _: serde_json::Value = match rpc_client
        .request(
            &authority_config.authority_rpc_url(),
            "get_authorized_skde_params",
            &GetAuthorizedSkdeParams,
            Id::Null,
        )
        .await
    {
        Ok(resp) => resp,
        Err(_e) => {
            serde_json::json!({ "skde_params": [] })
        }
    };

    // Get skde params from leader
    let (_leader_process, _leader_ports, _leader_config) =
        start_node(Role::Leader, 1, &mut temp_dirs).await;

    let response: GetSkdeParamsResponse = rpc_client
        .request(
            &authority_config.leader_cluster_rpc_url().clone().unwrap(),
            "get_skde_params",
            &GetSkdeParams,
            Id::Null,
        )
        .await
        .unwrap();

    // Read skde_params.json file from the filesystem
    let project_root = std::env::current_dir().expect("Failed to get current directory");
    let skde_params_path = project_root.join("data/authority/skde_params.json");

    // Check if file exists
    assert!(
        skde_params_path.exists(),
        "skde_params.json file does not exist: {:?}",
        skde_params_path
    );

    // Read file contents
    let file_content =
        fs::read_to_string(&skde_params_path).expect("Failed to read skde_params.json file");

    // Parse file content to JSON
    let file_json: serde_json::Value =
        serde_json::from_str(&file_content).expect("Failed to parse skde_params.json file as JSON");

    // Get SKDE params from the response
    let skde_params = response.into_skde_params();

    // Create a matching JSON structure to compare with file
    let response_json = serde_json::json!({
        "t": skde_params.t,
        "n": skde_params.n,
        "g": skde_params.g,
        "h": skde_params.h,
        "max_sequencer_number": skde_params.max_sequencer_number
    });

    // Compare entire JSON objects at once
    assert_eq!(
        file_json,
        response_json,
        "File content and API response do not match.\nFile: {}\nResponse: {}",
        serde_json::to_string_pretty(&file_json).unwrap(),
        serde_json::to_string_pretty(&response_json).unwrap()
    );

    // 5. Cleanup processes
    cleanup_existing_processes();
}
