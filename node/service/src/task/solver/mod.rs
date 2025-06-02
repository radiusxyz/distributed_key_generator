use super::{AppState, RpcParameter, Config};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_rpc::external::{GetKeyGeneratorList, GetKeyGeneratorRpcUrlListResponse};
use dkg_primitives::{KeyGeneratorList, AsyncTask};
use tokio::task::JoinHandle;

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];
    let leader = ctx.current_leader(false).map_err(|e| C::Error::from(e))?;
    fetch_key_generator_list(ctx, &leader.1).await?;

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server.init(&config.external_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));

    tracing::info!("External RPC server: {}", config.external_rpc_url);
    tracing::info!("Cluster RPC server: {}", config.cluster_rpc_url);

    Ok(handle)
}

pub async fn fetch_key_generator_list<C: AppState>(ctx: &C, leader_rpc_url: &str) -> Result<(), C::Error> {
    let response: GetKeyGeneratorRpcUrlListResponse = ctx
        .async_task()
        .request(
            leader_rpc_url.into(),
            <GetKeyGeneratorList as RpcParameter<C>>::method().into(),
            &GetKeyGeneratorList,
        )
        .await?;
    let key_generator_list: KeyGeneratorList<C::Address> = response.into();
    key_generator_list.put()?;
    Ok(())
}
