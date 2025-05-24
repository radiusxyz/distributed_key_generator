use super::{AppState, RpcParameter, Config};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_rpc::external::{GetTrustedSetup, GetTrustedSetupResponse, GetKeyGeneratorList, GetKeyGeneratorRpcUrlListResponse};
use dkg_primitives::{KeyGeneratorList, AsyncTask, TrustedSetupFor};
use tokio::task::JoinHandle;
use tracing::info;

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];
    let leader_rpc_url = ctx.leader_rpc_url().expect("Leader RPC URL not set");
    fetch_trusted_setup::<C>(ctx, &leader_rpc_url).await;
    fetch_key_generator_list(ctx, &leader_rpc_url).await?;

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server.init(&config.external_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(Box::pin(async move { server_handle.stopped().await; })));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(Box::pin(async move { server_handle.stopped().await; })));

    tracing::info!("External RPC server: {}", config.external_rpc_url);
    tracing::info!("Cluster RPC server: {}", config.cluster_rpc_url);

    Ok(handle)
}

// TODO: REFACTOR ME!
pub async fn fetch_trusted_setup<C: AppState>(ctx: &C, leader_rpc_url: &str) {
    loop {
        let result: Result<GetTrustedSetupResponse<C::Signature, TrustedSetupFor<C>>, C::Error> = ctx
            .async_task()
            .request(
                leader_rpc_url.to_string(),
                <GetTrustedSetup as RpcParameter<C>>::method().to_string(),
                GetTrustedSetup,
            )
            .await;

        match result {
            Ok(response) => {
                let GetTrustedSetupResponse { trusted_setup, signature } = response;

                match ctx.verify_signature(&signature, &trusted_setup, None) {
                    Ok(_signer_address) => { info!("Successfully fetched SKDE params from leader"); return }
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
