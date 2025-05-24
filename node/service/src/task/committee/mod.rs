use super::{AppState, RpcParameter, Config};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_rpc::external::{GetTrustedSetup, GetTrustedSetupResponse, AddKeyGenerator, RequestSubmitEncKey};
use dkg_primitives::{AsyncTask, TrustedSetupFor};
use tokio::task::JoinHandle;

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = Vec::new();
    let leader_rpc_url = ctx.leader_rpc_url().expect("Leader RPC URL not set");
    fetch_trusted_setup::<C>(ctx, &leader_rpc_url).await;
    add_key_generator::<C>(ctx, &config.cluster_rpc_url, &config.external_rpc_url, &leader_rpc_url);

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<RequestSubmitEncKey>()?
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
                    Ok(_signer_address) => {
                        tracing::info!("Successfully fetched trusted setup from leader");
                        return;
                    }
                    Err(e) => { panic!("Failed to verify trusted setup signature: {}", e) }
                }
            }
            Err(err) => { 
                tracing::warn!("Failed to fetch trusted setup from leader: {}, retrying in 1s...", err);
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
