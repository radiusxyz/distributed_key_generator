
use dkg_primitives::{
    to_signed_commitment, AsyncTask, DbManager, KeyGeneratorError, Operator, RuntimeError,
    SessionEvent, SessionId, SolverEvent,
};
use dkg_rpc::{
    submit_enc_key, EncKeyCommitment, FinalizedEncKeyPayload, GetHeartbeat, GetHeartbeatResponse,
    RequestSubmitEncKey, StartTimePayload, SubmitDecKey, SubmitEncKey, SyncFinalizedEncKeys,
    SyncStartTime,
};
use radius_sdk::validation_service::DkgValidation;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
    time::Duration,
};
use tracing::info;

use super::{Config, NodeConfig, RpcParameter};
use crate::{
    rpc::{default_cluster_rpc_server, default_external_rpc_server},
    run_session_worker,
};

mod key_manager;
use key_manager::KeyManager;

mod session_worker;
use session_worker::CommitteeSessionWorker;

pub async fn run_node<C: Config>(
    ctx: C,
    config: &NodeConfig,
    session_event_tx: Sender<SessionEvent<C::Signature, C::Address>>,
    session_event_rx: Receiver<SessionEvent<C::Signature, C::Address>>,
    solver_event_rx: Receiver<SolverEvent<C::Address>>,
) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = Vec::new();
    // 2. Get Solver Info
    let (_, solver_cluster_rpc_url, _) = ctx
        .validation_service()
        .get_solver_info()
        .await
        .expect("Failed to get solver info");
    info!("Solver info: {:?}", solver_cluster_rpc_url);

    let external_server = default_external_rpc_server(ctx.clone()).await?;
    let server_handle = external_server
        .register_rpc_method::<GetHeartbeat>()?
        .register_rpc_method::<RequestSubmitEncKey>()?
        .register_rpc_method::<SubmitEncKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitDecKey<C::Signature, C::Address>>()?
        .init(config.external_rpc_url.clone())
        .await?;
    handle.push(ctx.async_task().spawn_task(async move {
        server_handle.stopped().await;
    }));

    let cluster_server = default_cluster_rpc_server(ctx.clone()).await?;
    let server_handle = cluster_server
        .register_rpc_method::<SyncStartTime<C::Signature, C::Address>>()?
        .init(config.cluster_rpc_url.clone())
        .await?;
    handle.push(ctx.async_task().spawn_task(async move {
        server_handle.stopped().await;
    }));
    let sessions_per_round = ctx
        .validation_service()
        .get_session_per_round()
        .await
        .expect("Failed to get session per round");
    let collecting_duration = ctx
        .validation_service()
        .get_collecting_duration()
        .await
        .expect("Failed to get collecting duration");
    // Start the DKG worker
    let mut committee_session_worker = CommitteeSessionWorker::<C>::new(
        solver_cluster_rpc_url,
        session_event_tx,
        session_event_rx,
        1u64,
        sessions_per_round,
        Duration::from_millis(collecting_duration),
    );
    let session_duration = ctx
        .validation_service()
        .get_session_duration()
        .await
        .expect("Failed to get session duration");
    let cloned_ctx = ctx.clone();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_session_worker(
            cloned_ctx,
            &mut committee_session_worker,
            Duration::from_millis(session_duration),
        )
        .await
        {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);
    let mut key_manager = KeyManager::new(ctx.clone(), solver_event_rx);
    handle.push(ctx.async_task().spawn_task(async move {
        key_manager.run().await;
    }));
    Ok(handle)
}

/// Request submit encryption key for the initial session
pub fn init_genesis_session<C: Config>(
    ctx: C,
    operators: Vec<Operator<C::Address>>,
    session_id: SessionId,
    should_force_generating: bool,
) {
    if !ctx.is_leader(session_id) {
        return;
    }
    info!(
        "Initializing genesis session for DKG protocol: {:?}",
        session_id
    );
    let urls = operators
        .iter()
        .filter(|o| {
            if !should_force_generating {
                o.address() != ctx.address()
            } else {
                true
            }
        })
        .map(|o| o.external_rpc_url().to_string())
        .collect::<Vec<_>>();
    info!(
        "Broadcasting request to submit encryption key to {:?}",
        urls
    );
    ctx.async_task().multicast(
        urls,
        <RequestSubmitEncKey as RpcParameter<C>>::method().to_string(),
        RequestSubmitEncKey { session_id },
    );
}

pub async fn check_heartbeat<C: Config>(ctx: C, url: String) -> Result<bool, C::Error> {
    if let Ok(response) = ctx
        .async_task()
        .request(
            url,
            <GetHeartbeat as RpcParameter<C>>::method().to_string(),
            GetHeartbeat,
        )
        .await
    {
        let GetHeartbeatResponse { commitment } = response;
        if let Some(sender) = commitment.commitment.sender.clone() {
            let _ =
                ctx.verify_signature(&commitment.signature, &commitment.commitment, Some(sender))?;
        } else {
            return Err(KeyGeneratorError::InternalError("Sender is None".to_string()).into());
        }
        return Ok(true);
    }
    Ok(false)
}

pub fn sync_start_time<C: Config>(
    ctx: C,
    start_time: u128,
    operators: &Vec<Operator<C::Address>>,
) -> Result<(), C::Error> {
    let session_id = ctx
        .db_manager()
        .current_session()
        .map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    let commitment =
        to_signed_commitment(ctx.clone(), session_id, StartTimePayload::new(start_time))?;
    let urls = operators
        .iter()
        .filter(|o| o.address() != ctx.address())
        .map(|o| o.cluster_rpc_url().to_string())
        .collect::<Vec<_>>();
    info!("Broadcasting start time to {:?}", urls);
    ctx.async_task().multicast(
        urls,
        <SyncStartTime<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(),
        commitment,
    );
    Ok(())
}

/// Broadcast finalized encryption keys to the key generators including the solver
pub async fn sync_finalized_enc_keys<C: Config>(
    ctx: C,
    operators: &mut Vec<Operator<C::Address>>,
    commitments: Vec<EncKeyCommitment<C::Signature, C::Address>>,
    randomness: Vec<u8>,
    solver_url: String,
    session_id: SessionId,
) -> Result<(), C::Error> {
    let commitment = to_signed_commitment(
        ctx.clone(),
        session_id,
        FinalizedEncKeyPayload::<C::Signature, C::Address>::new(randomness, commitments),
    )?;
    let mut urls = operators
        .iter()
        .map(|o| o.cluster_rpc_url().to_string())
        .collect::<Vec<_>>();
    urls.push(solver_url);
    info!("Broadcasting finalized encryption keys to {:?}", urls);
    ctx.async_task().multicast(
        urls,
        <SyncFinalizedEncKeys<C::Signature, C::Address> as RpcParameter<C>>::method().to_string(),
        commitment,
    );
    Ok(())
}
