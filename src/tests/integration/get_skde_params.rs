use radius_sdk::json_rpc::client::{Id, RpcClient};

use crate::{
    rpc::{
        authority::GetAuthorizedSkdeParams,
        cluster::{GetSkdeParams, GetSkdeParamsResponse},
    },
    tests::utils::{init_test_environment, start_node},
    Role,
};

#[tokio::test]
async fn test_integration_get_skde_params() {
    // Initialize test environment
    init_test_environment("node registration test");

    // Vector to manage temporary directories
    let mut temp_dirs = Vec::new();

    // 1. Start authority, leader and committee nodes
    let (mut _authority_process, _authority_ports, authority_config) =
        start_node(Role::Authority, 9, &mut temp_dirs).await;

    // get authorized skde params from authority
    let rpc_client = RpcClient::new().unwrap();

    let response: serde_json::Value = match rpc_client
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

    println!("response:{:?}", response);

    // get skde params from leader

    let (mut _leader_process, _leader_ports, _leader_config) =
        start_node(Role::Leader, 0, &mut temp_dirs).await;

    let response: GetSkdeParamsResponse = rpc_client
        .request(
            &authority_config.leader_cluster_rpc_url().clone().unwrap(),
            "get_skde_params",
            &GetSkdeParams,
            Id::Null,
        )
        .await
        .unwrap();

    println!("response:{:?}", response);
}
