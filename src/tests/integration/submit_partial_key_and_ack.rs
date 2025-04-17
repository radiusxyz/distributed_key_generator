use bincode::serialize as serialize_to_bincode;
use radius_sdk::json_rpc::client::{Id, RpcClient};
use skde::key_generation::{generate_partial_key, prove_partial_key_validity};
use tokio::time::Duration;

use crate::{
    config::Role,
    rpc::{
        cluster::{GetKeyGeneratorList, PartialKeyPayload, SubmitPartialKeyResponse},
        common::create_signature,
    },
    tests::{test_helpers, utils},
    SessionId,
};

#[tokio::test]
async fn test_integration_submit_partial_key_and_ack() {
    // Setup test logging
    utils::setup_test_logging();

    // Get default test port configuration
    let ports = utils::TestPortConfig::default();

    // Create SKDE params for nodes
    let skde_params_leader = utils::create_skde_params();
    let skde_params_committee = utils::create_skde_params();

    // Create leader node configuration
    let (leader_config, _leader_temp_dir) = test_helpers::create_temp_config(
        Role::Leader,
        ports.leader.cluster,
        ports.leader.external,
        ports.leader.internal,
    );

    // Create follower node configuration
    let (committee_config, _committee_temp_dir) = test_helpers::create_temp_config(
        Role::Committee,
        ports.committee.cluster,
        ports.committee.external,
        ports.committee.internal,
    );

    // Run nodes as async tasks
    let _leader_handles =
        test_helpers::run_node(leader_config.clone(), skde_params_leader.clone()).await;
    let _committee_handles =
        test_helpers::run_node(committee_config.clone(), skde_params_committee.clone()).await;

    // Create RPC URLs for the follower
    let cluster_rpc_url = format!("http://127.0.0.1:{}", ports.committee.cluster);
    let external_rpc_url = format!("http://127.0.0.1:{}", ports.committee.external);

    // Create JSON-RPC client
    let rpc_client = RpcClient::new().unwrap();

    // follower and leader address
    let committee_address = committee_config.address();
    let _leader_address = leader_config.address();

    // Create parameters for add_key_generator using serde_json
    let add_key_generator = serde_json::json!({
        "message": {
            "address": committee_address.as_hex_string(),
            "cluster_rpc_url": cluster_rpc_url,
            "external_rpc_url": external_rpc_url
        }
    });

    // Register follower with leader
    rpc_client
        .request::<_, ()>(
            format!("http://127.0.0.1:{}", ports.leader.internal),
            "add_key_generator",
            &add_key_generator,
            Id::Number(1),
        )
        .await
        .unwrap();

    // Verify the follower is registered by querying the key generator list
    let response: serde_json::Value = rpc_client
        .request(
            format!("http://127.0.0.1:{}", ports.leader.cluster),
            "get_key_generator_list",
            &GetKeyGeneratorList,
            Id::Number(2),
        )
        .await
        .unwrap();

    // Check if the follower is registered
    let key_generator_list = response["key_generator_rpc_url_list"]
        .as_array()
        .expect("Invalid response format");

    let committee_found = key_generator_list
        .iter()
        .any(|kg| kg["address"].as_str().unwrap_or("") == committee_address.as_hex_string());

    assert!(committee_found, "Committee not found in key generator list");

    // Wait for the servers to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Create JSON-RPC client
    let rpc_client = RpcClient::new().unwrap();

    // follower's partial key generation
    let (secret_value, partial_key) = generate_partial_key(&skde_params_leader).unwrap();
    let partial_key_proof = prove_partial_key_validity(&skde_params_leader, &secret_value).unwrap();

    // Generate bincode for signature
    let payload = PartialKeyPayload {
        sender: committee_address.clone(),
        partial_key: partial_key.clone(),
        proof: partial_key_proof.clone(),
        submit_timestamp: 0,
        session_id: SessionId::default(),
    };
    let serialized_payload = serialize_to_bincode(&payload).unwrap();
    let signature = create_signature(&serialized_payload);

    // Create JSON parameter instead of using the struct directly
    let parameter = serde_json::json!({
        "signature": signature,
        "payload": {
            "sender": committee_address.clone(),
            "partial_key": partial_key,
            "proof": partial_key_proof,
            "submit_timestamp": 0,
            "session_id": SessionId::default()
        }
    });

    let response = rpc_client
        .request::<_, SubmitPartialKeyResponse>(
            format!("http://127.0.0.1:{}", ports.leader.cluster),
            "submit_partial_key",
            &parameter,
            Id::Number(2),
        )
        .await
        .unwrap();

    assert!(response.success);

    // TODO: ack_parktial_key should be checked
}
