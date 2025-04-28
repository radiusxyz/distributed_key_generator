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
        common::{PartialKeyPayload, SyncFinalizedPartialKeysPayload},
        solver::SolverSyncFinalizedPartialKeys,
    },
    state::AppState,
    types::*,
    utils::{
        log::{log_prefix_role_and_address, log_prefix_with_session_id},
        signature::create_signature,
    },
};
pub const THRESHOLD: usize = 1;

// Spawns a loop that periodically generates partial keys and aggregates them
pub fn run_single_key_generator(context: AppState) {
    let prefix = log_prefix_role_and_address(context.config());
    tokio::spawn(async move {
        let partial_key_generation_cycle_ms = context.config().partial_key_generation_cycle_ms();
        let partial_key_aggregation_cycle_ms = context.config().partial_key_aggregation_cycle_ms();

        info!(
            "{} Partial key generation cycle: {} ms, aggregation cycle: {} ms",
            prefix, partial_key_generation_cycle_ms, partial_key_aggregation_cycle_ms
        );

        loop {
            sleep(Duration::from_millis(partial_key_generation_cycle_ms)).await;
            let context = context.clone();

            let mut session_id = SessionId::get_mut().unwrap();
            let current_session_id = session_id.clone();
            let prefix: String = log_prefix_with_session_id(context.config(), &current_session_id);

            info!("{} Waiting to start session", prefix,);

            tokio::spawn(async move {
                let key_generator_rpc_url_list = KeyGeneratorList::get()
                    .unwrap()
                    .get_all_key_generator_rpc_url_list();

                if key_generator_rpc_url_list.is_empty() {
                    return;
                }

                let partial_key_address_list = PartialKeyAddressList::get_or(
                    current_session_id,
                    PartialKeyAddressList::default,
                )
                .unwrap();

                let partial_key_list = partial_key_address_list
                    .get_partial_key_list(current_session_id)
                    .unwrap();

                info!(
                    "{} Partial key list length: {}",
                    prefix,
                    partial_key_list.len()
                );

                if partial_key_address_list.is_empty() {
                    request_submit_partial_key(
                        &context,
                        key_generator_rpc_url_list,
                        current_session_id,
                    );
                    return;
                } else {
                    if let Err(err) =
                        broadcast_finalized_partial_keys(&context, current_session_id).await
                    {
                        error!(
                            "{} Error during partial key broadcasting: {:?}",
                            prefix, err
                        );
                        return;
                    }
                }

                session_id.increase_session_id();
                session_id.update().unwrap();
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

    let partial_keys = list.get_partial_key_list(session_id).unwrap();
    let partial_senders = list.to_vec();

    // TODO: Add to make actual signature from storage
    // TODO: Timestampes, signatures, etc. should be collected assigned to each partial key

    let submit_timestamps = vec![get_current_timestamp(); partial_keys.len()];
    let ack_timestamp = get_current_timestamp();

    let signatures: Vec<radius_sdk::signature::Signature> = partial_keys
        .iter()
        .zip(&partial_senders)
        .zip(&submit_timestamps)
        .map(|((key, sender), timestamp)| {
            let message = PartialKeyPayload {
                sender: sender.clone(),
                partial_key: key.clone(),
                submit_timestamp: *timestamp,
                session_id,
            };
            create_signature(context.config().signer(), &message).unwrap()
        })
        .collect();
    let payload = SyncFinalizedPartialKeysPayload {
        sender: context.config().address().clone(),
        partial_key_senders: partial_senders.clone(),
        partial_keys: partial_keys.clone(),
        session_id,
        submit_timestamps: submit_timestamps.clone(),
        signatures: signatures.clone(),
        ack_timestamp: ack_timestamp,
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
                        "Solver did not submit decryption key for session {} in time",
                        session_id.as_u64()
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
