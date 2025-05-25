use super::{AppState, RpcParameter, Config};
use crate::{DkgWorker, run_dkg_worker, rpc::{default_external_rpc_server, default_cluster_rpc_server}};
use dkg_primitives::{Event, AsyncTask, TrustedSetupFor};
use dkg_rpc::{SubmitDecKey, SubmitEncKey, GetTrustedSetup, GetTrustedSetupResponse};
use tokio::task::JoinHandle;
use tracing::{info, error};
use tokio::sync::mpsc::Receiver;

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config, rx: Receiver<Event<C::Signature, C::Address>>) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];
    let authority_rpc_url = config.maybe_authority_rpc_url.clone().expect("Authority RPC URL not set");
    let solver_rpc_url = config.maybe_solver_rpc_url.clone().expect("Solver RPC URL not set");
    fetch_trusted_setup::<C>(ctx, &authority_rpc_url).await;

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<SubmitEncKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitDecKey<C::Signature, C::Address>>()?
        .init(config.external_rpc_url.clone())
        .await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(&config.cluster_rpc_url).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));

    info!("External RPC server: {}", config.external_rpc_url);
    info!("Cluster RPC server: {}", config.cluster_rpc_url);

    let mut key_generator_worker = DkgWorker::<C>::new(solver_rpc_url, config.session_cycle, rx);
    let cloned_ctx = ctx.clone();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_dkg_worker(&cloned_ctx, &mut key_generator_worker).await {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);

    Ok(handle)
}

// TODO: REFACTOR ME!
pub async fn fetch_trusted_setup<C: AppState>(ctx: &C, authority_url: &str) {
    info!("Fetching SKDE params from authority: {}", authority_url);
    loop {
        let result: Result<GetTrustedSetupResponse<C::Signature, TrustedSetupFor<C>>, C::Error> = ctx
            .async_task()
            .request(
            authority_url.to_string(),
            <GetTrustedSetup as RpcParameter<C>>::method().to_string(),
            GetTrustedSetup,
            )
            .await;

        match result {
            Ok(response) => {
                let GetTrustedSetupResponse { trusted_setup, signature } = response;

                match ctx.verify_signature(&signature, &trusted_setup, None) {
                    Ok(_signer_address) => { 
                        info!("Successfully fetched SKDE params from authority");
                        return;
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