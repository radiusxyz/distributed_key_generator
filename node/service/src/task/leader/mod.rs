use super::{AppState, SkdeParams, RpcClient, RpcClientError, Id, RpcParameter, DkgAppState, Config, Error};
use crate::rpc::{default_internal_rpc_server, default_external_rpc_server, default_cluster_rpc_server};
use dkg_node_primitives::{Address, Signature};
use dkg_rpc::{cluster::SubmitDecryptionKey, external::{GetSkdeParams, GetSkdeParamsResponse}};
use dkg_primitives::TaskSpawner;

pub async fn run_node(ctx: &mut DkgAppState, config: Config) -> Result<(), Error> {
    let authority_rpc_url = config.maybe_authority_rpc_url.expect("Authority RPC URL not set");
    let skde_params = fetch_skde_params(ctx, &authority_rpc_url).await;
    ctx.with_skde_params(skde_params);
    
    let internal_server = default_internal_rpc_server(ctx).await?;
    let server_handle = internal_server.init(config.internal_rpc_url).await?;
    ctx.task_spawner().spawn_task(Box::pin(async move {
        server_handle.stopped().await;
    }));

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<SubmitDecryptionKey<Signature, Address>>()?
        .init(config.external_rpc_url.clone())
        .await?;
    ctx.task_spawner().spawn_task(Box::pin(async move {
        server_handle.stopped().await;
    }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(config.cluster_rpc_url).await?;
    ctx.task_spawner().spawn_task(Box::pin(async move {
        server_handle.stopped().await;
    }));

    Ok(())
}

// TODO: REFACTOR ME!
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