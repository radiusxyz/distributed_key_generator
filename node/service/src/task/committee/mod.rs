use super::{Config, RpcParameter, NodeConfig};
use crate::{rpc::{default_cluster_rpc_server, default_external_rpc_server}, run_session_worker, run_genesis_session};
use dkg_rpc::{AddKeyGenerator, RequestSubmitEncKey, SubmitEncKey, SubmitDecKey, EncKeyCommitment, FinalizedEncKeyPayload, SyncFinalizedEncKeys, submit_enc_key};
use dkg_primitives::{AuthService, AsyncTask, RuntimeEvent, SessionId, KeyGenerator, Commitment, SignedCommitment};
use tokio::{task::JoinHandle, sync::mpsc::Receiver};
use tracing::info;

mod worker;
use worker::CommitteeWorker;

pub async fn run_node<C: Config>(ctx: &mut C, config: NodeConfig, rx: Receiver<RuntimeEvent<C::Signature, C::Address>>) -> Result<Vec<JoinHandle<()>>, C::Error> {
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

    // Start the DKG worker
    let initial_key_generators = ctx.auth_service().get_key_generators(0).await.expect("Failed to get initial key generators");
    let mut key_generator_worker = CommitteeWorker::<C>::new(solver_cluster_rpc_url, rx, initial_key_generators.clone(), config.round_look_ahead, 1u64);
    let cloned_ctx = ctx.clone();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_genesis_session(&cloned_ctx, 0, config.threshold, initial_key_generators).await {
            panic!("Error on genesis session: {}", e);
        }
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

/// Request submit encryption key for the initial session
pub fn init<C: Config>(
    ctx: &C,
    key_generators: Vec<KeyGenerator<C::Address>>,
    session_id: SessionId,
) {
    if !ctx.is_leader() { return; }
    let urls = key_generators.iter().map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    ctx.async_task().multicast(urls, <RequestSubmitEncKey as RpcParameter<C>>::method().to_string(), RequestSubmitEncKey { session_id });
}

/// Broadcast finalized encryption keys to the key generators including the solver
pub async fn sync_finalized_enc_keys<C: Config>(
    ctx: &C,
    key_generators: &mut Vec<KeyGenerator<C::Address>>,
    commitments: Vec<EncKeyCommitment<C::Signature, C::Address>>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let payload = FinalizedEncKeyPayload::<C::Signature, C::Address>::new(commitments);
    let bytes = serde_json::to_vec(&payload).map_err(|e| C::Error::from(e))?;
    let commitment = Commitment::new(bytes.into(), Some(ctx.address()), session_id);
    let signature = ctx.sign(&commitment)?;
    let mut urls = key_generators.iter().map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    urls.push(solver_url);
    info!("Broadcasting finalized encryption keys to {:?}", urls);
    ctx.async_task().multicast(urls, <SyncFinalizedEncKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), SignedCommitment { signature, commitment });
    Ok(())
}
