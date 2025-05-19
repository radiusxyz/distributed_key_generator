use super::{AppState, SkdeParams, RpcClient, RpcClientError, Id, RpcParameter, DkgAppState, Config, Error};
use crate::rpc::{initialize_internal_rpc_server, initialize_external_rpc_server, initialize_cluster_rpc_server};
use dkg_rpc::external::{GetSkdeParams, GetSkdeParamsResponse};

pub async fn run_node(ctx: &mut DkgAppState, config: Config) -> Result<(), Error> {
    let leader_rpc_url = ctx.leader_rpc_url().expect("Leader RPC URL not set");
    let skde_params = fetch_skde_params(ctx, &leader_rpc_url).await;
    ctx.with_skde_params(skde_params);
    initialize_internal_rpc_server(ctx, &config.internal_rpc_url).await?;
    initialize_external_rpc_server(ctx, &config.external_rpc_url).await?;
    initialize_cluster_rpc_server(ctx, &config.cluster_rpc_url).await?;

    Ok(())
}

pub async fn fetch_skde_params<C: AppState>(ctx: &C, authority_url: &str) -> SkdeParams {
    let client = RpcClient::new().unwrap();
    let result: Result<GetSkdeParamsResponse<C::Signature>, RpcClientError> = client
        .request(
            authority_url,
            <GetSkdeParams as RpcParameter<C>>::method(),
            &GetSkdeParams,
            Id::Null,
        )
        .await;

    match result {
        Ok(response) => {
            let signed = response.signed_skde_params;

            match ctx.verify_signature(&signed.signature, &signed.params, None) {
                Ok(_signer_address) => { signed.params }
                Err(e) => { panic!("Failed to verify SKDE params signature: {}", e) }
            }
        }
        Err(err) => { panic!("Failed to fetch SkdeParams from authority: {}", err) }
    }
}