use super::{Config, RpcParameter, NodeConfig};
use crate::{rpc::{default_cluster_rpc_server, default_external_rpc_server}, run_session_worker};
use dkg_rpc::{submit_enc_key, DbManager, EncKeyCommitment, FinalizedEncKeyPayload, GetHeartbeat, GetHeartbeatResponse, RequestSubmitEncKey, StartTimePayload, SubmitDecKey, SubmitEncKey, SyncFinalizedEncKeys, SyncStartTime};
use dkg_primitives::{AsyncTask, AuthService, AuthServiceError, KeyGenerator, KeyServiceError, RuntimeError, RuntimeEvent, SessionId, to_signed_commitment};
use tokio::{task::JoinHandle, sync::mpsc::{Sender, Receiver}};
use tracing::info;

mod worker;
use worker::CommitteeWorker;

pub async fn run_node<C: Config>(ctx: &mut C, config: &NodeConfig, event_tx: Sender<RuntimeEvent<C::Signature, C::Address>>, event_rx: Receiver<RuntimeEvent<C::Signature, C::Address>>) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = Vec::new();
    // 2. Get Solver Info
    let (_, solver_cluster_rpc_url, _) = ctx.auth_service().get_solver_info().await.expect("Failed to get solver info");
    info!("Solver info: {:?}", solver_cluster_rpc_url);

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server
        .register_rpc_method::<GetHeartbeat>()?
        .register_rpc_method::<RequestSubmitEncKey>()?
        .register_rpc_method::<SubmitEncKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitDecKey<C::Signature, C::Address>>()?
        .init(config.external_rpc_url.clone())
        .await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server
        .register_rpc_method::<SyncStartTime<C::Signature, C::Address>>()?
        .init(config.cluster_rpc_url.clone()).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));

    // Start the DKG worker
    let mut key_generator_worker = CommitteeWorker::<C>::new(solver_cluster_rpc_url, event_tx, event_rx, config.round_look_ahead, 1u64);
    let cloned_ctx = ctx.clone();
    let session_duration_millis = config.session_duration_millis();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_session_worker(&cloned_ctx, &mut key_generator_worker, session_duration_millis).await {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);

    Ok(handle)
}

/// Request submit encryption key for the initial session
pub fn init_genesis_session<C: Config>(
    ctx: &C,
    key_generators: Vec<KeyGenerator<C::Address>>,
    session_id: SessionId,
    should_force_generating: bool,
) {    
    if !ctx.is_leader(session_id) { info!("Only leader can initialize genesis session"); return; }
    info!("Initializing genesis session for DKG protocol: {:?}", session_id);
    let urls = key_generators.iter().filter(|kg| {
        if !should_force_generating {
            kg.address() != ctx.address()
        } else {
            true
        }
    }).map(|kg| kg.external_rpc_url().to_string()).collect::<Vec<_>>();
    info!("Broadcasting request to submit encryption key to {:?}", urls);
    ctx.async_task().multicast(urls, <RequestSubmitEncKey as RpcParameter<C>>::method().to_string(), RequestSubmitEncKey { session_id });
}

pub async fn check_heartbeat<C: Config>(ctx: &C, url: String, registered_address: C::Address) -> Result<bool, C::Error> {
    if let Ok(response) = ctx
        .async_task()
        .request(url, <GetHeartbeat as RpcParameter<C>>::method().to_string(), GetHeartbeat)
        .await {
            let GetHeartbeatResponse { commitment } = response;
            if let Some(sender) = commitment.commitment.sender.clone() {    
                let maybe_committee = ctx.verify_signature(&commitment.signature, &commitment.commitment, Some(sender))?;
                if maybe_committee != registered_address {
                    return Err(AuthServiceError::AnyError("Sender is not in the committee".to_string()).into());
                }
            } else {
                return Err(KeyServiceError::InternalError("Sender is None".to_string()).into());
            }
            return Ok(true); 
        } 
        Ok(false)
}

pub fn sync_start_time<C: Config>(ctx: &C, start_time: u128, key_generators: &Vec<KeyGenerator<C::Address>>) -> Result<(), C::Error> {
    let session_id = ctx.db_manager().current_session().map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    let commitment = to_signed_commitment(ctx, session_id, StartTimePayload::new(start_time))?;
    let urls = key_generators.iter().filter(|kg| kg.address() != ctx.address()).map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    info!("Broadcasting start time to {:?}", urls);
    ctx.async_task().multicast(urls, <SyncStartTime<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), commitment);
    Ok(())
}

/// Broadcast finalized encryption keys to the key generators including the solver
pub async fn sync_finalized_enc_keys<C: Config>(
    ctx: &C,
    key_generators: &mut Vec<KeyGenerator<C::Address>>,
    commitments: Vec<EncKeyCommitment<C::Signature, C::Address>>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let commitment = to_signed_commitment(ctx, session_id, FinalizedEncKeyPayload::<C::Signature, C::Address>::new(commitments))?;
    let mut urls = key_generators.iter().map(|kg| kg.cluster_rpc_url().to_string()).collect::<Vec<_>>();
    urls.push(solver_url);
    info!("Broadcasting finalized encryption keys to {:?}", urls);
    ctx.async_task().multicast(urls, <SyncFinalizedEncKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(), commitment);
    Ok(())
}
