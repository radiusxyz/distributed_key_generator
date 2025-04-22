use std::time::Duration;

use radius_sdk::json_rpc::{
    client::{Id, RpcClient},
    server::RpcParameter,
};
use sha2::{Digest, Sha256}; // SHA-256
use sha3::digest::{ExtendableOutput, Update, XofReader}; // Shake256
use sha3::Shake256;
use skde::{
    delay_encryption::{solve_time_lock_puzzle, SkdeParams},
    key_aggregation::aggregate_key,
    key_generation::{generate_uv_pair, PartialKey as SkdePartialKey},
    BigUint,
};
use tokio::time::sleep;
use tracing::info;

use crate::{rpc::cluster::RequestSubmitPartialKey, state::AppState, types::*, utils::AddressExt};

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

                let partial_key_address_list = PartialKeyAddressList::get_or(
                    current_session_id,
                    PartialKeyAddressList::default,
                )
                .unwrap();

                let partial_key_list = partial_key_address_list
                    .get_partial_key_list(current_session_id)
                    .unwrap();

                if key_generator_rpc_url_list.is_empty() {
                    return;
                } else {
                    if partial_key_address_list.is_empty() {
                        info!(
                            "[{}] Requested to submit partial key for session id: {}",
                            context.config().address().to_short(),
                            current_session_id.as_u64()
                        );

                        // Request partial keys from other generators when partial_key_address_list is empty
                        request_submit_partial_key(key_generator_rpc_url_list, current_session_id);
                    }
                }

                info!(
                    "[{}] Partial key address list: {:?}",
                    context.config().address().to_short(),
                    partial_key_address_list
                );

                info!(
                    "[{}] Partial key list: {:?}",
                    context.config().address().to_short(),
                    partial_key_list
                );

                let previous_session_id = SessionId::from(current_session_id.as_u64() - 1);
                let randomness = match DecryptionKey::get(previous_session_id) {
                    Ok(key) => {
                        info!(
                            "[{}] Using decryption key from previous session (previous_session_id = {}) as randomness",
                            context.config().address().to_short(),
                            previous_session_id.as_u64()
                        );
                        key.as_string().into_bytes()
                    }
                    Err(err) => {
                        tracing::warn!(
                            "[{}] Failed to get decryption key for previous_session_id = {}: {}; falling back to default randomness",
                            context.config().address().to_short(),
                            previous_session_id.as_u64(),
                            err
                        );
                        b"default-randomness".to_vec()
                    }
                };

                // All nodes execute the key selection and derivation
                let skde_params = context.skde_params().clone();
                let mut selected_keys = select_random_partial_keys(&partial_key_list, &randomness);

                let derived_key = derive_partial_key(&selected_keys, &skde_params);
                selected_keys.push(derived_key);

                let skde_aggregated_key = aggregate_key(&skde_params, &selected_keys);
                let aggregated_key = AggregatedKey::new(skde_aggregated_key.clone());
                aggregated_key.put(current_session_id).unwrap();

                info!(
                    "[{}] Completed to generate encryption key - session id: {:?} / encryption key: {:?}",
                    context.config().address().to_short(),
                    current_session_id,
                    skde_aggregated_key.u
                );

                // TODO: This puzzle should be solved by the Solver node
                let secure_key =
                    solve_time_lock_puzzle(&skde_params, &skde_aggregated_key).unwrap();

                let decryption_key = DecryptionKey::new(secure_key.sk.clone());
                decryption_key.put(current_session_id).unwrap();

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

// Selects a randomized subset of indices based on input randomness
fn select_ordered_indices(n: usize, randomness: &[u8]) -> Vec<usize> {
    assert!(n >= 1, "Need at least 1 partial key to proceed");

    // Special case: when there is only one partial key, return index 0
    // Having only one key is insecure and should not be used in production
    if n == 1 {
        return vec![0];
    }

    let first_byte = randomness[0] as usize;
    // TODO: k must be less than maximum number of partial key generators
    let k = (first_byte % (n - 1)) + 1;

    let mut indices: Vec<usize> = (0..n).collect();
    let mut state = randomness.to_vec();

    for i in (1..n).rev() {
        let mut hasher = Sha256::new();
        sha2::Digest::update(&mut hasher, &state);
        sha2::Digest::update(&mut hasher, &[i as u8]);
        let hash = hasher.finalize();
        let rand_byte = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let j = (rand_byte % (i as u64 + 1)) as usize;
        indices.swap(i, j);
        state = hash.to_vec();
    }

    indices[..k].to_vec()
}

fn select_random_partial_keys(
    partial_keys: &Vec<SkdePartialKey>,
    randomness: &[u8],
) -> Vec<SkdePartialKey> {
    let indices = select_ordered_indices(partial_keys.len(), randomness);
    indices.iter().map(|&i| partial_keys[i].clone()).collect()
}

// Uses SHAKE256 as a hash-to-biguint function for deriving randomness
// SHAKE256 is chosen here because it produces an arbitrary-length digest,
// which allows us to generate uniformly distributed big integers of desired bit size.
fn shake256_to_biguint(input: &[u8], size: usize) -> BigUint {
    let mut hasher = Shake256::default();
    hasher.update(input);
    let mut reader = hasher.finalize_xof();
    let mut buf = vec![0u8; size];
    reader.read(&mut buf);
    BigUint::from_bytes_le(&buf)
}

// Derives a partial key from selected partial keys
fn derive_partial_key(selected_keys: &Vec<SkdePartialKey>, params: &SkdeParams) -> SkdePartialKey {
    let n = BigUint::parse_bytes(params.n.as_bytes(), 10).unwrap();
    let max_sequencer_number =
        BigUint::parse_bytes(params.max_sequencer_number.as_bytes(), 10).unwrap();

    // Creates a virtual partial key based on hashed combination
    let mut h_input = Vec::new();
    for key in selected_keys {
        h_input.extend(serde_json::to_vec(key).unwrap());
    }

    let n_half = &n / 2u32;
    let n_half_over_max_sequencer_number = &n / (2u32 * &max_sequencer_number);

    let gen = |label: &[u8]| {
        let mut input = h_input.clone();
        input.push(label[0]);
        shake256_to_biguint(&input, 32)
    };

    let r_h = gen(b"r") % &n_half_over_max_sequencer_number;
    let s_h = gen(b"s") % &n_half_over_max_sequencer_number;
    let k_h = gen(b"k") % &n_half;
    // u, v = g^(r + s), h^{(r + s) * n} * (1 + n)^s
    let uv_pair = generate_uv_pair(params, &(&r_h + &s_h), &s_h)
        .expect("Failed to generate UV pair for partial key");

    // y, w = g^k, g^{k * n} * (1 + n)^r
    let yw_pair =
        generate_uv_pair(params, &k_h, &r_h).expect("Failed to generate YW pair for partial key");

    SkdePartialKey {
        u: uv_pair.u,
        v: uv_pair.v,
        y: yw_pair.u,
        w: yw_pair.v,
    }
}
