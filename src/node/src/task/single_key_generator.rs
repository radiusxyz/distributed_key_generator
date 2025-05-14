use std::time::Duration;

use radius_sdk::json_rpc::{
    client::{Id, RpcClient, RpcClientError},
    server::{RpcError, RpcParameter},
};
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{
    get_current_timestamp,
    rpc::{
        cluster::{ClusterSyncFinalizedPartialKeys, RequestSubmitPartialKey},
        common::SyncFinalizedPartialKeysPayload,
        solver::SolverSyncFinalizedPartialKeys,
    },
    state::AppState,
    types::*,
    utils::{
        log::{log_prefix_role_and_address, log_prefix_with_session_id},
        signature::create_signature,
    }, Error,
};
pub const THRESHOLD: usize = 1;

// Spawns a loop that periodically generates partial keys and aggregates them
pub fn run_single_key_generator(context: AppState) {
    let prefix = log_prefix_role_and_address(context.config());
    let generation_cycle = context.config().generation_cycle;
    PartialKeyAddressList::initialize(SessionId::from(0)).expect("Failed to initialize partial key address list");
    info!(
        "{} Partial key generation cycle: {} ms, aggregation cycle: {} ms",
        prefix, 
        generation_cycle, 
        context.config().aggregation_cycle
    );

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(generation_cycle)).await;
            let context = context.clone();

            let mut current_session = SessionId::get_mut().unwrap();
            let prefix: String = log_prefix_with_session_id(context.config(), &*current_session);
            info!("{} 🔑🗝️🔑 Waiting to start session 🔑🗝️🔑", prefix,);

            tokio::spawn(async move {
                let key_generator_rpc_url_list = KeyGeneratorList::get()
                    .unwrap()
                    .get_all_key_generator_rpc_url_list();

                if key_generator_rpc_url_list.is_empty() {
                    return;
                }

                let partial_key_address_list = PartialKeyAddressList::get_or(
                    *current_session,
                    PartialKeyAddressList::default,
                )
                .unwrap();

                let partial_key_submissions = partial_key_address_list
                    .get_partial_key_list(*current_session)
                    .unwrap_or_default();

                info!(
                    "{} Partial key list length: {}",
                    prefix,
                    partial_key_submissions.len()
                );

                if partial_key_address_list.is_empty() {
                    request_submit_partial_key(
                        &context,
                        key_generator_rpc_url_list,
                        *current_session,
                    );
                    return;
                } else {
                    if let Err(err) =
                        broadcast_finalized_partial_keys(&context, *current_session).await
                    {
                        error!(
                            "{} Error during partial key broadcasting: {:?}",
                            prefix, err
                        );
                        return;
                    }
                }
                
                current_session.next_mut().unwrap();
                PartialKeyAddressList::initialize(current_session.clone()).map_err(|e| Error::Database(e)).unwrap();
                current_session.update().unwrap();
            });
        }
    });
}

pub fn request_submit_partial_key(
    context: &AppState,
    key_generator_rpc_url_list: Vec<String>,
    session_id: SessionId,
) {
    let prefix = log_prefix_with_session_id(context.config(), &session_id);

    tokio::spawn({
        async move {
            let parameter = RequestSubmitPartialKey { session_id };

            match RpcClient::new() {
                Ok(rpc_client) => {
                    match rpc_client
                        .multicast(
                            key_generator_rpc_url_list.clone(),
                            RequestSubmitPartialKey::method(),
                            &parameter,
                            Id::Null,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!("{} Successfully requested submit partial key", prefix);
                        }
                        Err(err) => {
                            error!(
                                "{} Failed to multicast RequestSubmitPartialKey: {}",
                                prefix, err,
                            );
                        }
                    }
                }
                Err(err) => {
                    error!("{} Failed to create RpcClient: {}", prefix, err,);
                }
            }
        }
    });
}

pub async fn broadcast_finalized_partial_keys(
    context: &AppState,
    session_id: SessionId,
) -> Result<(), RpcError> {
    let prefix = log_prefix_with_session_id(context.config(), &session_id);

    // TODO: needs wait to collect partial keys, instead of loop
    let list = loop {
        if let Ok(list) = PartialKeyAddressList::get(session_id) {
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

    // TODO: Add to make actual signature
    // TODO: Timestampes, signatures, etc. should be collected assigned to each partial key
    let partial_key_submissions = list.get_partial_key_list(session_id).unwrap();

    // TODO: Replace actual PartialKeySubmissions
    // let partial_key_submissions = partial_keys
    //     .iter()
    //     .zip(&partial_senders)
    //     .zip(&signatures)
    //     .map(|((key, sender), signature)| SubmitPartialKey {
    //         signature: signature.clone(),
    //         payload: PartialKeyPayload {
    //             partial_key: key.clone(),
    //             sender: sender.clone(),
    //             submit_timestamp: submit_timestamps[i],
    //             session_id,
    //         },
    //     })
    //     .collect();

    let payload: SyncFinalizedPartialKeysPayload = SyncFinalizedPartialKeysPayload {
        sender: context.config().address().clone(),
        partial_key_submissions,
        session_id,
        ack_timestamp: get_current_timestamp(),
    };

    let signature = create_signature(context.config().signer(), &payload).unwrap();
    let message = ClusterSyncFinalizedPartialKeys {
        signature: signature.clone(),
        payload: payload.clone(),
    };

    let peers = KeyGeneratorList::get()?.get_all_key_generator_rpc_url_list();
    let rpc_client = RpcClient::new()?;
    let prefix = log_prefix_with_session_id(context.config(), &session_id);

    if let Err(err) = rpc_client
        .multicast(
            peers,
            ClusterSyncFinalizedPartialKeys::method(),
            &message,
            Id::Null,
        )
        .await
    {
        error!("{} Failed to broadcast partial key list: {:?}", prefix, err);
    } else {
        info!(
            "{} Successfully broadcasted finalized partial key list to cluster",
            prefix
        );
    }

    let message = SolverSyncFinalizedPartialKeys { signature, payload };

    let solver_url = context.config().solver_solver_rpc_url().clone().unwrap();
    let rpc_client = RpcClient::new()?;
    let response = rpc_client
        .request::<_, ()>(
            solver_url.clone(),
            SolverSyncFinalizedPartialKeys::method(),
            &message,
            Id::Null,
        )
        .await;

    match response {
        Ok(_) => info!("{} Solver at {} responded successfully", prefix, solver_url),
        Err(err) => error!("{} Failed to respond to solver: {:?}", prefix, err),
    }
    Ok(())
}

pub async fn wait_for_decryption_key(
    context: &AppState,
    session_id: SessionId,
    timeout_secs: u64,
) -> Result<DecryptionKey, RpcClientError> {
    let poll_interval = Duration::from_secs(1);
    let mut waited = 0;
    let prefix = log_prefix_with_session_id(context.config(), &session_id);

    loop {
        match DecryptionKey::get(session_id) {
            Ok(key) => {
                info!("{} Received decryption key", prefix);
                return Ok(key);
            }
            Err(_) => {
                if waited >= timeout_secs {
                    error!("{} Timeout waiting for decryption key", prefix);
                    return Err(RpcClientError::Response(format!(
                        "Solver did not submit decryption key for session {:?} in time",
                        session_id
                    )));
                }

                debug!(
                    "{} Still waiting for decryption key (waited: {}s)",
                    prefix, waited
                );

                sleep(poll_interval).await;
                waited += 1;
            }
        }
    }
}
