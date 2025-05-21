use super::{AppState, SkdeParams, RpcParameter, DkgAppState, Config, Error};
use crate::{DkgWorker, run_dkg_worker};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_node_primitives::{Address, Signature};
use dkg_primitives::Event;
use dkg_rpc::{SubmitDecryptionKey, SubmitPartialKey, GetSkdeParams, GetSkdeParamsResponse};
use dkg_primitives::TaskSpawner;
use tokio::task::JoinHandle;
use tracing::{info, error};
use tokio::sync::mpsc::Receiver;

pub async fn run_node(ctx: &mut DkgAppState, config: Config, rx: Receiver<Event<Signature, Address>>) -> Result<Vec<JoinHandle<()>>, Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];
    let authority_rpc_url = config.maybe_authority_rpc_url.clone().expect("Authority RPC URL not set");
    let solver_rpc_url = config.maybe_solver_rpc_url.clone().expect("Solver RPC URL not set");
    let skde_params = fetch_skde_params(ctx, &authority_rpc_url).await;
    ctx.with_skde_params(skde_params);

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<SubmitPartialKey<Signature, Address>>()?
        .register_rpc_method::<SubmitDecryptionKey<Signature, Address>>()?
        .init(config.external_rpc_url.clone())
        .await?;
    handle.push(ctx.task_spawner().spawn_task(Box::pin(async move {
        server_handle.stopped().await;
    })));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.task_spawner().spawn_task(Box::pin(async move {
        server_handle.stopped().await;
    })));

    info!("External RPC server: {}", config.external_rpc_url);
    info!("Cluster RPC server: {}", config.cluster_rpc_url);

    let mut key_generator_worker = DkgWorker::new(solver_rpc_url, config.session_cycle, rx);
    let cloned_ctx = ctx.clone();
    let worker_handle = ctx.spawn_task(async move {
        if let Err(e) = run_dkg_worker(&cloned_ctx, &mut key_generator_worker).await {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);

    Ok(handle)
}

// TODO: REFACTOR ME!
pub async fn fetch_skde_params<C: AppState>(ctx: &C, authority_url: &str) -> SkdeParams {
    info!("Fetching SKDE params from authority: {}", authority_url);
    loop {
        let result: Result<GetSkdeParamsResponse<C::Signature>, C::Error> = ctx.request(
            authority_url.to_string(),
            <GetSkdeParams as RpcParameter<C>>::method().to_string(),
            GetSkdeParams,
            )
            .await;

        match result {
            Ok(response) => {
                let signed = response.signed_skde_params;

                match ctx.verify_signature(&signed.signature, &signed.params, None) {
                    Ok(_signer_address) => { 
                        info!("Successfully fetched SKDE params from authority");
                        return signed.params
                    }
                    Err(e) => { panic!("Failed to verify SKDE params signature: {}", e) }
                }
            }
            Err(err) => { 
                error!("Failed to fetch SkdeParams from authority: {}, retrying in 1s...", err);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}