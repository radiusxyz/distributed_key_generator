use super::{AppState, SkdeParams, RpcParameter, DkgAppState, Config, Error};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_rpc::external::{GetSkdeParams, GetSkdeParamsResponse, AddKeyGenerator, RequestSubmitPartialKey};
use dkg_primitives::AsyncTask;
use tokio::task::JoinHandle;

pub async fn run_node(ctx: &mut DkgAppState, config: Config) -> Result<Vec<JoinHandle<()>>, Error> {
    let mut handle: Vec<JoinHandle<()>> = Vec::new();
    let leader_rpc_url = ctx.leader_rpc_url().expect("Leader RPC URL not set");
    let skde_params = fetch_skde_params(ctx, &leader_rpc_url).await;
    ctx.with_skde_params(skde_params);
    add_key_generator::<DkgAppState>(ctx, &config.cluster_rpc_url, &config.external_rpc_url, &leader_rpc_url);

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<RequestSubmitPartialKey>()?
        .init(&config.external_rpc_url)
        .await?;
    handle.push(ctx.async_task().spawn_task(Box::pin(async move { server_handle.stopped().await; })));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(Box::pin(async move { server_handle.stopped().await; })));

    tracing::info!("External RPC server: {}", config.external_rpc_url);
    tracing::info!("Cluster RPC server: {}", config.cluster_rpc_url);

    Ok(handle)
}

// TODO: REFACTOR ME!
pub async fn fetch_skde_params<C: AppState>(ctx: &C, leader_rpc_url: &str) -> SkdeParams {
    loop {
        let result: Result<GetSkdeParamsResponse<C::Signature>, C::Error> = ctx
            .async_task()
            .request(
                leader_rpc_url.to_string(),
                <GetSkdeParams as RpcParameter<C>>::method().to_string(),
                GetSkdeParams,
            )
            .await;

        match result {
            Ok(response) => {
                let signed = response.signed_skde_params;

                match ctx.verify_signature(&signed.signature, &signed.params, None) {
                    Ok(_signer_address) => {
                        tracing::info!("Successfully fetched SKDE params from leader");
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

fn add_key_generator<C: AppState>(
    ctx: &C,
    cluster_rpc_url: &str, 
    external_rpc_url: &str,
    leader_rpc_url: &str,
) {
    let param = AddKeyGenerator::new(ctx.address(), cluster_rpc_url.to_string(), external_rpc_url.to_string());
    ctx.async_task().multicast(vec![leader_rpc_url.to_string()], <AddKeyGenerator<C::Address> as RpcParameter<C>>::method().to_string(), param);
}
