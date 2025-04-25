use std::time::Duration;

use bincode::serialize;
use radius_sdk::json_rpc::{
    client::{Id, RpcClient, RpcClientError},
    server::{RpcError, RpcParameter},
};
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{
    rpc::{
        cluster::{self, RequestSubmitPartialKey},
        solver,
    },
    state::AppState,
    types::*,
    utils::{AddressExt, *},
};
pub const THRESHOLD: usize = 1;

// TODO: Decouple logic according to roles.
// Spawns a loop that periodically generates partial keys and aggregates them
pub fn run_single_key_generator(context: AppState) {
    tokio::spawn(async move {
        let partial_key_generation_cycle_ms = context.config().partial_key_generation_cycle_ms();
        let partial_key_aggregation_cycle_ms = context.config().partial_key_aggregation_cycle_ms();

        info!(
            "[{}] Partial key generation cycle: {} seconds, Partial key aggregation cycle: {} seconds",
            context.config().address().to_short(),
            partial_key_generation_cycle_ms,
            partial_key_aggregation_cycle_ms
        );

        loop {
            // Necessary sleep to prevent lock timeout errors
            sleep(Duration::from_millis(partial_key_generation_cycle_ms)).await;
            let context = context.clone();

            let mut session_id = SessionId::get_mut().unwrap();
            let current_session_id = session_id.clone();
            info!("Current session id: {}", current_session_id.as_u64());

            tokio::spawn(async move {
                // Get RPC URLs of other key generators excluding leader
                let key_generator_rpc_url_list = KeyGeneratorList::get()
                    .unwrap()
                    .get_other_key_generator_rpc_url_list(&context.config().address());

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

                if partial_key_address_list.is_empty() {
                    info!(
                        "[{}] Requested to submit partial key for session id: {}",
                        context.config().address().to_short(),
                        current_session_id.as_u64()
                    );
                    // Request partial keys from other generators when partial_key_address_list is empty
                    request_submit_partial_key(key_generator_rpc_url_list, current_session_id);
                } else {
                    if let Err(e) = broadcast_partial_keys(&context, current_session_id).await {
                        tracing::error!("Error during partial key broadcasting: {:?}", e);
                        return;
                    }
                }

                info!(
                    "[{}] Partial key list len: {:?}",
                    context.config().address().to_short(),
                    partial_key_list.len()
                );

                // All nodes share the same aggregation function
                let skde_aggregated_key =
                    perform_randomized_aggregation(&context, current_session_id, &partial_key_list);

                // TODO: This puzzle should be solved by the Solver node
                let decryption_key =
                    calculate_decryption_key(&context, current_session_id, &skde_aggregated_key)
                        .unwrap();

                info!(
                    "[{}] Complete to get decryption key - session id: {:?} / decryption key: {:?}",
                    context.config().address().to_short(),
                    current_session_id,
                    decryption_key.as_string()
                );

                // Increment session ID after successful key generation
                session_id.increase_session_id();
                session_id.update().unwrap();
            });
        }
    });
}

pub fn request_submit_partial_key(key_generator_rpc_url_list: Vec<String>, session_id: SessionId) {
    tokio::spawn(async move {
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
                        info!(
                            "Successfully requested submit partial key for session id: {}",
                            session_id.as_u64()
                        );
                    }
                    Err(err) => {
                        error!(
                            "Failed to multicast RequestSubmitPartialKey: {} for session id: {}, urls: {:?}",
                            err,
                            session_id.as_u64(),
                            key_generator_rpc_url_list
                        );
                    }
                }
            }
            Err(err) => {
                error!(
                    "Failed to create RpcClient: {} for session id: {}",
                    err,
                    session_id.as_u64()
                );
            }
        }
    });
}

pub async fn broadcast_partial_keys(
    context: &AppState,
    session_id: SessionId,
) -> Result<(), RpcError> {
    // TODO: needs wait to collect partial keys, instead of loop
    let list = loop {
        if let Ok(list) = PartialKeyAddressList::get(session_id) {
            let current_count = list.len();
            debug!(
                "[{}] PartialKeyList - session_id: {}, collected: {}, threshold: {}",
                context.config().address().to_short(),
                session_id.as_u64(),
                current_count,
                THRESHOLD
            );

            if current_count >= THRESHOLD {
                info!(
                    "[{}] Threshold met for session {} ({} >= {}), preparing to broadcast",
                    context.config().address().to_short(),
                    session_id.as_u64(),
                    current_count,
                    THRESHOLD
                );
                break list;
            }
        } else {
            debug!(
                "[{}] PartialKeyList not yet available for session_id: {}",
                context.config().address().to_short(),
                session_id.as_u64()
            );
        }
        sleep(Duration::from_secs(1)).await;
    };

    let partial_keys = list.get_partial_key_list(session_id).unwrap();
    let partial_senders = list.to_vec();

    // TODO: Add to make actual signature
    // TODO: Timestampes, signatures, etc. should be collected assigned to   each partial key
    let signatures = partial_keys
        .iter()
        .zip(&partial_senders)
        .map(|(key, sender)| {
            let message = (sender, key, session_id);
            let encoded = bincode::serialize(&message).unwrap();
            create_signature(&encoded)
        })
        .collect();
    let submit_timestamps = vec![get_current_timestamp(); partial_keys.len()];

    let payload = cluster::SyncPartialKeysPayload {
        partial_key_senders: partial_senders,
        partial_keys,
        session_id,
        submit_timestamps,
        signatures,
        ack_timestamp: get_current_timestamp(),
    };

    let signature = create_signature(&serialize(&payload)?);
    let message = cluster::SyncPartialKeys { signature, payload };
    let sender = context.config().signer().address();
    let peers = KeyGeneratorList::get()?.get_other_key_generator_rpc_url_list(&sender);
    let rpc_client = RpcClient::new()?;
    if let Err(e) = rpc_client
        .multicast(
            peers,
            cluster::SyncPartialKeys::method(),
            &message,
            Id::Null,
        )
        .await
    {
        error!(
            "Failed to broadcast partial key list for session {}: {:?}",
            session_id.as_u64(),
            e
        );
    } else {
        info!(
            "[{}] Successfully broadcasted partial key list to cluster for session {}",
            context.config().address().to_short(),
            session_id.as_u64()
        );
    }

    let solver_url = context.config().solver_solver_rpc_url().clone().unwrap();
    let rpc_client = RpcClient::new()?;
    let response = rpc_client
        .request::<_, ()>(
            solver_url.clone(),
            solver::SyncPartialKeys::method(),
            &message,
            Id::Null,
        )
        .await;

    match response {
        Ok(_) => info!("Solver at {} responded successfully", solver_url),
        Err(e) => error!(
            "Solver at {} failed to respond (session {}): {:?}",
            solver_url,
            session_id.as_u64(),
            e
        ),
    }
    Ok(())
}

pub async fn wait_for_decryption_key(
    session_id: SessionId,
    timeout_secs: u64,
) -> Result<DecryptionKey, RpcClientError> {
    let poll_interval = Duration::from_secs(1);
    let mut waited = 0;

    loop {
        match DecryptionKey::get(session_id) {
            Ok(key) => {
                info!(
                    "[LEADER] Received decryption key for session_id: {}",
                    session_id.as_u64()
                );
                return Ok(key);
            }
            Err(_) => {
                if waited >= timeout_secs {
                    error!(
                        "[LEADER] Timeout waiting for decryption key (session_id: {})",
                        session_id.as_u64()
                    );
                    return Err(RpcClientError::Response(format!(
                        "Solver did not submit decryption key for session {} in time",
                        session_id.as_u64()
                    )));
                }

                debug!(
                    "[LEADER] Still waiting for decryption key (session_id: {}, waited: {}s)",
                    session_id.as_u64(),
                    waited
                );

                sleep(poll_interval).await;
                waited += 1;
            }
        }
    }
}
