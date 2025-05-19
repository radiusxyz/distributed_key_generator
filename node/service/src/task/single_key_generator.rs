use std::time::Duration;
use tokio::time::sleep;
use radius_sdk::json_rpc::{client::{Id, RpcClient, RpcClientError}, server::{RpcError, RpcParameter}};
use tracing::{debug, error, info};
use dkg_primitives::{
    SyncFinalizedPartialKeysPayload, 
    AppState, 
    KeyGeneratorList, 
    PartialKeyAddressList,
    SessionId,
    DecryptionKey,
};
use dkg_rpc::{
    cluster::{ClusterSyncFinalizedPartialKeys, RequestSubmitPartialKey},
    solver::SolverSyncFinalizedPartialKeys,
};

pub const THRESHOLD: usize = 1;

// TODO: REFACTOR ME!

// Spawns a loop that periodically generates partial keys and aggregates them
pub fn run_single_key_generator<C: AppState>(ctx: C, session_cycle: u64, solver_rpc_url: &'static str) {
    let _ = ctx.clone().spawn_task(Box::pin(
        async move {
            PartialKeyAddressList::<C::Address>::initialize(SessionId::from(0)).unwrap();
    
            info!("{} Session cycle: {} ms", ctx.log_prefix(), session_cycle);
    
            loop {
                sleep(Duration::from_millis(session_cycle)).await;
    
                let mut session_id = SessionId::get_mut().unwrap();
                let current_session_id = session_id.clone();
                let ctx_clone = ctx.clone();

                // TODO: Remove unwrap
                let next_session_id = current_session_id.next().unwrap();
                PartialKeyAddressList::<C::Address>::initialize(next_session_id).unwrap();
    
                info!(
                    "{} 🔑🗝️🔑 Waiting to start on session {:?} 🔑🗝️🔑",
                    ctx_clone.log_prefix(),
                    current_session_id
                );
    
                tokio::spawn(async move {
                    let key_generator_rpc_url_list = KeyGeneratorList::<C::Address>::get()
                        .unwrap()
                        .get_all_key_generator_rpc_url_list();
    
                    if key_generator_rpc_url_list.is_empty() {
                        return;
                    }
    
                    let partial_key_address_list = PartialKeyAddressList::<C::Address>::get_or(
                        current_session_id,
                        || PartialKeyAddressList::<C::Address>::new(),
                    )
                    .unwrap();
    
                    let partial_key_submissions = partial_key_address_list
                        .get_partial_key_list::<C>(current_session_id)
                        .unwrap_or_default();
    
                    info!(
                        "{} Partial key list length: {}",
                        ctx_clone.log_prefix(),
                        partial_key_submissions.len()
                    );
    
                    if partial_key_address_list.is_empty() {
                        request_submit_partial_key::<C>(
                            ctx_clone,
                            key_generator_rpc_url_list,
                            current_session_id,
                        );
                        return;
                    } else {
                        if let Err(err) =
                            broadcast_finalized_partial_keys::<C>(&ctx_clone, solver_rpc_url, current_session_id).await
                        {
                            error!(
                                "{} Error during partial key broadcasting: {:?}",
                                ctx_clone.log_prefix(), err
                            );
                            return;
                        }
                    }
    
                    session_id.next_mut().unwrap();
                    session_id.update().unwrap();
                });
            }
        }
    ));
}

pub fn request_submit_partial_key<C: AppState>(
    ctx: C,
    key_generator_rpc_url_list: Vec<String>,
    session_id: SessionId,
) {
    let _ = ctx.clone().spawn_task(Box::pin(
        async move {
            let parameter = RequestSubmitPartialKey { session_id };

            match RpcClient::new() {
                Ok(rpc_client) => {
                    match rpc_client
                        .multicast(
                            key_generator_rpc_url_list.clone(),
                            <RequestSubmitPartialKey as RpcParameter<C>>::method(),
                            &parameter,
                            Id::Null,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!("{} Successfully requested submit partial key on session {:?}", ctx.log_prefix(), session_id);
                        }
                        Err(err) => {
                            error!(
                                "{} Failed to multicast RequestSubmitPartialKey: {:?}",
                                ctx.log_prefix(), err,
                            );
                        }
                    }
                }
                Err(err) => {
                    error!("{} Failed to create RpcClient: {:?}", ctx.log_prefix(), err);
                }
            }
        }
    ));
}

pub async fn broadcast_finalized_partial_keys<C: AppState>(
    ctx: &C,
    solver_rpc_url: &'static str,
    session_id: SessionId,
) -> Result<(), RpcError> {
    let prefix = ctx.log_prefix();

    // TODO: needs wait to collect partial keys, instead of loop
    let list = loop {
        if let Ok(list) = PartialKeyAddressList::<C::Address>::get(session_id) {
            let current_count = list.len();
            debug!(
                "{} PartialKeyList collected: {}, threshold: {}",
                prefix, current_count, THRESHOLD
            );

            if current_count >= THRESHOLD {
                info!(
                    "{} Threshold met ({} >= {}), preparing to broadcast",
                    prefix, current_count, THRESHOLD
                );
                break list;
            }
        } else {
            debug!("{} PartialKeyList not yet available", prefix);
        }
        // Should be declared as constant
        sleep(Duration::from_millis(100)).await;
    };

    let partial_key_submissions = list.get_partial_key_list::<C>(session_id).unwrap();

    let payload = SyncFinalizedPartialKeysPayload::<C::Signature, C::Address>::new(
        ctx.address(),
        partial_key_submissions,
        session_id,
    );

    let signature = ctx.sign(&payload).unwrap();
    let message = ClusterSyncFinalizedPartialKeys {
        signature: signature.clone(),
        payload: payload.clone(),
    };

    let peers = KeyGeneratorList::<C::Address>::get()
        .unwrap()
        .get_all_key_generator_rpc_url_list();

    let rpc_client = RpcClient::new()?;

    if let Err(err) = rpc_client
        .multicast(
            peers,
            <ClusterSyncFinalizedPartialKeys<C::Signature, C::Address> as RpcParameter<C>>::method(),
            &message,
            Id::Null,
        )
        .await
    {
        error!("{} Failed to broadcast partial key list: {:?}", ctx.log_prefix(), err);
    } else {
        info!(
            "{} Successfully broadcasted finalized partial key list to cluster on session {:?}",
            ctx.log_prefix(), session_id
        );
    }

    let message = SolverSyncFinalizedPartialKeys { signature, payload };
    let rpc_client = RpcClient::new()?;
    let response = rpc_client
        .request::<_, ()>(
            solver_rpc_url.clone(),
            <SolverSyncFinalizedPartialKeys<C::Signature, C::Address> as RpcParameter<C>>::method(),
            &message,
            Id::Null,
        )
        .await;

    match response {
        Ok(_) => info!("{} Solver at {} responded successfully", ctx.log_prefix(), solver_rpc_url),
        Err(err) => error!("{} Failed to respond to solver: {:?}", prefix, err),
    }
    Ok(())
}

pub async fn wait_for_decryption_key<C: AppState>(
    ctx: &C,
    session_id: SessionId,
    timeout_secs: u64,
) -> Result<DecryptionKey, C::Error> {
    let poll_interval = Duration::from_secs(1);
    let mut waited = 0;
    loop {
        match DecryptionKey::get(session_id) {
            Ok(key) => {
                info!("{} Received decryption key on session {:?}", ctx.log_prefix(), session_id);
                return Ok(key);
            }
            Err(_) => {
                if waited >= timeout_secs {
                    error!("{} Timeout waiting for decryption key on session {:?}", ctx.log_prefix(), session_id);
                    return Err(C::Error::from(RpcClientError::Response(format!(
                        "Solver did not submit decryption key for session {:?} in time",
                        session_id
                    ))));
                }

                debug!(
                    "{} Still waiting for decryption key on session {:?} (waited: {}s)",
                    ctx.log_prefix(), session_id, waited
                );

                sleep(poll_interval).await;
                waited += 1;
            }
        }
    }
}
