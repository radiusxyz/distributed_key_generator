use super::{Config, RpcParameter, NodeConfig};
use crate::{rpc::{default_external_rpc_server, default_cluster_rpc_server}, SessionWorker, run_session_worker};
use dkg_rpc::{AddKeyGenerator, RequestSubmitEncKey, SubmitEncKey, SubmitDecKey};
use dkg_primitives::{AuthService, AsyncTask, Event};
use tokio::{task::JoinHandle, sync::mpsc::Receiver};

pub async fn run_node<C: Config>(ctx: &mut C, config: NodeConfig, rx: Receiver<Event<C::Signature, C::Address>>) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = Vec::new();
    // Solver must be known at this point
    let (_, solver_cluster_rpc_url, _) = ctx.auth_service().get_solver_info().await.unwrap();
    // Get the current leader for session 0
    let leader = ctx.current_leader(false).map_err(|e| C::Error::from(e))?;
    add_key_generator::<C>(ctx, &config.cluster_rpc_url, &config.external_rpc_url, &leader.1);

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<RequestSubmitEncKey>()?
        .register_rpc_method::<SubmitEncKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitDecKey<C::Signature, C::Address>>()?
        .init(&config.external_rpc_url)
        .await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));

    tracing::info!("External RPC server: {}", config.external_rpc_url);
    tracing::info!("Cluster RPC server: {}", config.cluster_rpc_url);

    // Start the DKG worker
    let initial_key_generators = ctx.auth_service().get_key_generators(0).await.expect("Failed to get initial key generators");
    let mut key_generator_worker = SessionWorker::<C>::new(ctx, solver_cluster_rpc_url, rx, initial_key_generators);
    let cloned_ctx = ctx.clone();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_session_worker(&cloned_ctx, &mut key_generator_worker, config.session_duration_millis()).await {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);

    Ok(handle)
}

fn add_key_generator<C: Config>(
    ctx: &C,
    cluster_rpc_url: &str, 
    external_rpc_url: &str,
    leader_rpc_url: &str,
) {
    let param = AddKeyGenerator::new(false, ctx.address(), cluster_rpc_url.to_string(), external_rpc_url.to_string());
    ctx.async_task().multicast(vec![leader_rpc_url.to_string()], <AddKeyGenerator<C::Address> as RpcParameter<C>>::method().to_string(), param);
}
