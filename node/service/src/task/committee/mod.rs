use super::{AppState, SkdeParams, RpcClient, RpcClientError, Id, RpcParameter, DkgAppState, Config, Error};
use crate::rpc::{initialize_internal_rpc_server, initialize_external_rpc_server, initialize_cluster_rpc_server};
use dkg_rpc::{external::{GetSkdeParams, GetSkdeParamsResponse}, cluster::{GetKeyGeneratorList, GetKeyGeneratorRpcUrlListResponse}};
use dkg_primitives::KeyGeneratorList;

pub async fn run_node(ctx: &mut DkgAppState, config: Config) -> Result<(), Error> {
    let leader_rpc_url = ctx.leader_rpc_url().expect("Leader RPC URL not set");
    let skde_params = fetch_skde_params(ctx, &leader_rpc_url).await;
    ctx.with_skde_params(skde_params);
    fetch_key_generator_list::<DkgAppState>(&leader_rpc_url).await?;
    initialize_internal_rpc_server(ctx, &config.internal_rpc_url).await?;
    initialize_external_rpc_server(ctx, &config.external_rpc_url).await?;
    initialize_cluster_rpc_server(ctx, &config.cluster_rpc_url).await?;

    Ok(())
}

pub async fn fetch_skde_params<C: AppState>(ctx: &C, leader_rpc_url: &str) -> SkdeParams {
    let client = RpcClient::new().unwrap();
    loop {
        let result: Result<GetSkdeParamsResponse<C::Signature>, RpcClientError> = client
            .request(
                leader_rpc_url,
                <GetSkdeParams as RpcParameter<C>>::method(),
                &GetSkdeParams,
                Id::Null,
            )
            .await;

        match result {
            Ok(response) => {
                let signed = response.signed_skde_params;

                match ctx.verify_signature(&signed.signature, &signed.params, None) {
                    Ok(_signer_address) => {
                        return signed.params
                    }
                    Err(e) => { panic!("Failed to verify SKDE params signature: {}", e) }
                }
            }
            Err(err) => { 
                tracing::warn!("Failed to fetch SkdeParams from leader: {}, retrying in 1s...", err);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

pub async fn fetch_key_generator_list<C: AppState>(leader_rpc_url: &str) -> Result<(), Error> {
    let rpc_client = RpcClient::new()?;
    let response: GetKeyGeneratorRpcUrlListResponse = rpc_client
        .request(
            leader_rpc_url,
            <GetKeyGeneratorList as RpcParameter<C>>::method(),
            &GetKeyGeneratorList,
            Id::Null,
        )
        .await?;
    let key_generator_list: KeyGeneratorList<C::Address> = response.into();
    key_generator_list.put()?;
    Ok(())
}
