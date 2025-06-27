use super::{Config, NodeConfig, run_session_worker};
use crate::rpc::{default_external_rpc_server, default_cluster_rpc_server};
use dkg_rpc::{DecKeyPayload, SubmitDecKeyResponse, SubmitDecKey};
use dkg_primitives::{AsyncTask, DecKey, SessionId, SignedCommitment, KeyService, RuntimeError, to_signed_commitment};
use radius_sdk::json_rpc::server::RpcParameter;
use tracing::info;
use tokio::task::JoinHandle;
use tokio::sync::mpsc::Receiver;
use dkg_primitives::RuntimeEvent;

mod worker;
use worker::SolverWorker;

pub async fn run_node<C: Config>(ctx: &mut C, config: &NodeConfig, rx: Receiver<RuntimeEvent<C::Signature, C::Address>>) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];

    let external_server = default_external_rpc_server(ctx).await?;
    let server_handle = external_server.init(config.external_rpc_url.clone()).await?;
    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));
    
    let cluster_server = default_cluster_rpc_server(ctx).await?;
    let server_handle = cluster_server.init(config.cluster_rpc_url.clone()).await?;

    handle.push(ctx.async_task().spawn_task(async move { server_handle.stopped().await; }));

    let mut worker = SolverWorker::<C>::new(rx);
    let cloned_ctx = ctx.clone();
    let session_duration_millis = config.session_duration_millis();
    let worker_handle = ctx.async_task().spawn_task(async move {
        if let Err(e) = run_session_worker(&cloned_ctx, &mut worker, session_duration_millis).await {
            // TODO: Spawn critical task to start DKG worker
            panic!("Error running DKG worker: {}", e);
        }
    });
    handle.push(worker_handle);


    Ok(handle)
}

/// Solve based on the given encryption keys and create a signed commitment
pub fn do_solve_key<C: Config>(
    ctx: &C,
    session_id: SessionId,
    enc_key: &Vec<u8>,
) -> Result<SignedCommitment<C::Signature, C::Address>, C::Error> {
    info!("Start solving at session: {:?}", session_id);
    let (dec_key, solve_at) = ctx.key_service().gen_dec_key(enc_key).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    info!("End solving at session: {:?}", session_id);
    DecKey::new(dec_key.clone()).put(session_id).map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    let commitment = to_signed_commitment(ctx, session_id, DecKeyPayload::new(dec_key, solve_at))?;
    Ok(commitment)
}

pub async fn submit_dec_key<C: Config>(ctx: &C, session_id: SessionId, commitment: SignedCommitment<C::Signature, C::Address>) -> Result<(), C::Error> {
    let leader_rpc_url = ctx.current_leader(session_id, false)?.1;
    // TODO: Handle Error
    let _: SubmitDecKeyResponse = ctx
        .async_task()
        .request(
            leader_rpc_url,
            <SubmitDecKey::<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
            SubmitDecKey(commitment),
        )
        .await?;
    Ok(())
}
