

use dkg_primitives::{
    to_signed_commitment, AsyncTask, DecKey, KeyGenerator, RuntimeError, SessionEvent, SessionId,
    SignedCommitment,
};
use dkg_rpc::{DecKeyPayload, SubmitDecKey, SubmitDecKeyResponse};
use radius_sdk::{json_rpc::server::RpcParameter, validation_service::DkgValidation};
use tokio::{sync::mpsc::Receiver, task::JoinHandle, time::Duration};
use tracing::info;

use super::{Config, NodeConfig};
use crate::rpc::{default_cluster_rpc_server, default_external_rpc_server};
mod worker;
use worker::SolverWorker;

pub async fn run_node<C: Config>(
    ctx: C,
    config: &NodeConfig,
    rx: Receiver<SessionEvent<C::Signature, C::Address>>,
) -> Result<Vec<JoinHandle<()>>, C::Error> {
    let mut handle: Vec<JoinHandle<()>> = vec![];
    let external_server = default_external_rpc_server(ctx.clone()).await?;
    let server_handle = external_server
        .init(config.external_rpc_url.clone())
        .await?;
    handle.push(ctx.async_task().spawn_task(async move {
        server_handle.stopped().await;
    }));

    let cluster_server = default_cluster_rpc_server(ctx.clone()).await?;
    let server_handle = cluster_server.init(config.cluster_rpc_url.clone()).await?;

    handle.push(ctx.async_task().spawn_task(async move {
        server_handle.stopped().await;
    }));
    let session_duration = ctx
        .validation_service()
        .get_session_duration()
        .await
        .expect("Failed to get session duration");
    let mut worker =
        SolverWorker::<C>::new(ctx.clone(), rx, Duration::from_millis(session_duration));
    let worker_handle = ctx
        .async_task()
        .spawn_task(async move { worker.run().await });
    handle.push(worker_handle);

    Ok(handle)
}

/// Solve based on the given encryption keys and create a signed commitment
pub async fn do_solve_key<C: Config>(
    ctx: C,
    session_id: SessionId,
    enc_key: Vec<u8>,
    randomness: Vec<u8>,
) -> Result<SignedCommitment<C::Signature, C::Address>, C::Error> {
    info!("Start solving at session: {:?}", session_id);
    let (dec_key, solve_at) = ctx
        .key_generator()
        .read()
        .await
        .gen_dec_key(&enc_key)
        .map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    info!("End solving at session: {:?}", session_id);
    DecKey::new(dec_key.clone())
        .put(session_id)
        .map_err(|e| RuntimeError::AnyError(Box::new(e)))?;
    let commitment = to_signed_commitment(
        ctx.clone(),
        session_id,
        DecKeyPayload::new(enc_key, randomness, dec_key, solve_at),
    )?;
    Ok(commitment)
}

pub async fn submit_dec_key<C: Config>(
    ctx: C,
    session_id: SessionId,
    commitment: SignedCommitment<C::Signature, C::Address>,
) -> Result<(), C::Error> {
    let leader_rpc_url = ctx.current_leader(session_id, false)?.1;
    // TODO: Handle Error
    let _: SubmitDecKeyResponse = ctx
        .async_task()
        .request(
            leader_rpc_url,
            <SubmitDecKey<C::Signature, C::Address> as RpcParameter<C>>::method().into(),
            SubmitDecKey(commitment),
        )
        .await?;
    Ok(())
}
