use std::time::Duration;

use radius_sdk::json_rpc::{
    client::{Id, RpcClient},
    server::RpcParameter,
};
use tokio::time::sleep;
use tracing::info;

use crate::{
    rpc::cluster::RequestSubmitPartialKey,
    state::AppState,
    types::*,
    utils::{AddressExt, *},
};

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
                    calculate_decryption_key(&context, &skde_aggregated_key, current_session_id)
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
                        tracing::error!(
                            "Failed to multicast RequestSubmitPartialKey: {} for session id: {}, urls: {:?}",
                            err,
                            session_id.as_u64(),
                            key_generator_rpc_url_list
                        );
                    }
                }
            }
            Err(err) => {
                tracing::error!(
                    "Failed to create RpcClient: {} for session id: {}",
                    err,
                    session_id.as_u64()
                );
            }
        }
    });
}
