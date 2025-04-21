use std::time::{Duration, SystemTime, UNIX_EPOCH};

use radius_sdk::{
    json_rpc::{
        client::{Id, RpcClient},
        server::RpcParameter,
    },
    signature::Address,
};
use sha2::{Digest, Sha256}; // SHA-256
use sha3::digest::{ExtendableOutput, Update, XofReader}; // Shake256
use sha3::Shake256;
use skde::{
    delay_encryption::{solve_time_lock_puzzle, SkdeParams},
    key_aggregation::{aggregate_key, AggregatedKey as SkdeAggregatedKey},
    key_generation::{generate_uv_pair, PartialKey as SkdePartialKey},
    BigUint,
};
use tokio::time::sleep;
use tracing::info;

use crate::{
    rpc::cluster::{RequestSubmitPartialKey, SyncAggregatedKey},
    state::AppState,
    types::*,
    utils::AddressExt,
};

// TODO: Decoupling logic according to the roles.
// Spawns a loop that periodically generates partial keys and aggregates
pub fn run_single_key_generator(context: AppState) {
    tokio::spawn(async move {
        let partial_key_generation_cycle = context.config().partial_key_generation_cycle();
        let partial_key_aggregation_cycle = context.config().partial_key_aggregation_cycle();

        loop {
            sleep(Duration::from_secs(partial_key_generation_cycle)).await;
            let context = context.clone();

            let mut session_id = SessionId::get_mut().unwrap();
            let current_session_id = session_id.clone();
            info!("Current session id: {}", current_session_id.as_u64());

            tokio::spawn(async move {
                sleep(Duration::from_secs(partial_key_aggregation_cycle)).await;
                let skde_params = context.skde_params().clone();

                let partial_key_address_list = PartialKeyAddressList::get_or(
                    current_session_id,
                    PartialKeyAddressList::default,
                )
                .unwrap();

                let participant_addresses = partial_key_address_list.to_vec();
                let partial_key_list = partial_key_address_list
                    .get_partial_key_list(current_session_id)
                    .unwrap();

                let other_key_generator_rpc_url_list = KeyGeneratorList::get()
                    .unwrap()
                    .get_other_key_generator_rpc_url_list(&context.config().address());

                if partial_key_list.is_empty() {
                    tracing::info!(
                        "No partial key list found for session id: {}",
                        current_session_id.as_u64()
                    );
                    if !other_key_generator_rpc_url_list.is_empty() {
                        info!(
                            "other_key_generator_rpc_url_list: {:?}",
                            other_key_generator_rpc_url_list
                        );

                        tracing::info!(
                            "[{}] Requested to submit partial key for session id: {}",
                            context.config().address().to_short(),
                            current_session_id.as_u64()
                        );

                        request_submit_partial_key(
                            other_key_generator_rpc_url_list,
                            current_session_id,
                        );
                    } else {
                        tracing::warn!(
                            "No key generator and partial key list found for session id: {}",
                            current_session_id.as_u64()
                        );
                    }
                    return;
                }

                let previous_session_id = SessionId::from(current_session_id.as_u64() - 1);
                let randomness = DecryptionKey::get(previous_session_id)
                    .map(|key| {
                        tracing::info!(
                            "[{}] Using decryption key from previous session (previous_session_id = {}) as randomness",
                            context.config().address().to_short(),
                            previous_session_id.as_u64()
                        );
                        key.as_string().into_bytes()
                    })
                    .unwrap_or_else(|err| {
                        tracing::warn!(
                            "[{}] Failed to get decryption key for previous_session_id = {}: {}; falling back to default randomness",
                            context.config().address().to_short(),
                            previous_session_id.as_u64(),
                            err
                            );
                        b"default-randomness".to_vec()
                    });

                let mut selected_keys = select_random_partial_keys(&partial_key_list, &randomness);

                let derived_key = derive_partial_key(&selected_keys, &skde_params);
                selected_keys.push(derived_key);

                let skde_aggregated_key = aggregate_key(&skde_params, &selected_keys);
                let aggregated_key = AggregatedKey::new(skde_aggregated_key.clone());
                aggregated_key.put(current_session_id).unwrap();

                tracing::info!(
                    "Completed to generate encryption key - session id: {:?} / encryption key: {:?}",
                    current_session_id,
                    skde_aggregated_key.u
                );

                sync_aggregated_key(
                    current_session_id,
                    skde_aggregated_key.clone(),
                    participant_addresses,
                    context.config().signer().address(),
                );

                let secure_key =
                    solve_time_lock_puzzle(&skde_params, &skde_aggregated_key).unwrap();
                let decryption_key = DecryptionKey::new(secure_key.sk.clone());
                decryption_key.put(current_session_id).unwrap();

                tracing::info!(
                    "[{}] Complete to get decryption key - session id: {:?} / decryption key: {:?}",
                    context.config().address().to_short(),
                    current_session_id,
                    decryption_key
                );

                // when successfully generated partial key, increase session_id
                session_id.increase_session_id();
                session_id.update().unwrap();
            });
        }
    });
}

pub fn request_submit_partial_key(
    other_key_generator_rpc_url_list: Vec<String>,
    session_id: SessionId,
) {
    tokio::spawn(async move {
        let parameter = RequestSubmitPartialKey { session_id };

        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                other_key_generator_rpc_url_list,
                RequestSubmitPartialKey::method(),
                &parameter,
                Id::Null,
            )
            .await
            .unwrap();
    });
}

// Multicasts the aggregated key to all other key generators
// TODO: Each node performs the aggregation independently.
pub fn sync_aggregated_key(
    session_id: SessionId,
    aggregated_key: SkdeAggregatedKey,
    participant_addresses: Vec<Address>,
    my_address: &Address,
) {
    let other_key_generator_rpc_url_list = KeyGeneratorList::get()
        .unwrap()
        .get_other_key_generator_rpc_url_list(my_address);

    tokio::spawn(async move {
        let parameter = SyncAggregatedKey {
            session_id,
            aggregated_key,
            participant_addresses,
        };

        let rpc_client = RpcClient::new().unwrap();
        rpc_client
            .multicast(
                other_key_generator_rpc_url_list,
                SyncAggregatedKey::method(),
                &parameter,
                Id::Null,
            )
            .await
            .unwrap();
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

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
