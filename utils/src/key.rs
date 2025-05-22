use dkg_primitives::{
    EncKey, DecKey, AppState, KeyGenerationError, SessionId, TraceExt,
};
use sha2::{Digest, Sha256};
use sha3::{digest::{ExtendableOutput, Update, XofReader}, Shake256};
use skde::{
    delay_encryption::{decrypt, encrypt, solve_time_lock_puzzle, SkdeParams},
    key_aggregation::{aggregate_key, AggregatedKey},
    key_generation::{generate_uv_pair, PartialKey},
    BigUint,
};
use tracing::info;

// TODO: A more robust mechanism to handle delayed or missing solve operations should be designed.
pub fn get_enc_key<C: AppState>(
    context: &C,
    session_id: SessionId,
    partial_key_list: &[PartialKey],
) -> Result<EncKey, C::Error> {
    let skde_params = context.skde_params().clone();
    let randomness = get_randomness(session_id);
    let mut selected_keys = select_random_partial_keys(partial_key_list, &randomness);
    let derived_key = derive_partial_key(&selected_keys, &skde_params);
    selected_keys.push(derived_key);
    let enc_key: EncKey = aggregate_key(&skde_params, &selected_keys).into();
    enc_key.put(session_id)?;

    info!("Completed to generate encryption key - session id: {:?}",session_id);

    Ok(enc_key)
}

pub fn get_dec_key<C: AppState>(
    context: &C,
    session_id: SessionId,
    enc_key: &AggregatedKey,
) -> Result<DecKey, C::Error> {
    let skde_params = context.skde_params();

    // TODO: Timeout
    let secure_key = solve_time_lock_puzzle(&skde_params, enc_key).map_err(|err| {
        KeyGenerationError::InternalError(format!("Failed to solve time lock puzzle: {:?}", err))
    })?;

    let dec_key = DecKey::new(secure_key.sk.clone());

    dec_key.put(session_id).map_err(|err| {
        KeyGenerationError::InvalidPartialKey(format!("Failed to store decryption key: {:?}", err))
    })?;

    Ok(dec_key)
}

pub fn select_random_partial_keys(
    partial_keys: &[PartialKey],
    randomness: &[u8],
) -> Vec<PartialKey> {
    let indices = select_ordered_indices(partial_keys.len(), randomness);
    indices.iter().map(|&i| partial_keys[i].clone()).collect()
}

// Uses SHAKE256 as a hash-to-biguint function for deriving randomness
// SHAKE256 is chosen here because it produces an arbitrary-length digest,
// which allows us to generate uniformly distributed big integers of desired bit size.
pub fn shake256_to_biguint(input: &[u8], size: usize) -> BigUint {
    let mut hasher = Shake256::default();
    hasher.update(input);
    let mut reader = hasher.finalize_xof();
    let mut buf = vec![0u8; size];
    reader.read(&mut buf);
    BigUint::from_bytes_le(&buf)
}

// Derives a partial key from selected partial keys
pub fn derive_partial_key(
    selected_keys: &Vec<PartialKey>,
    params: &SkdeParams,
) -> PartialKey {
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

    PartialKey {
        u: uv_pair.u,
        v: uv_pair.v,
        y: yw_pair.u,
        w: yw_pair.v,
    }
}

// Selects a randomized subset of indices based on input randomness
pub fn select_ordered_indices(n: usize, randomness: &[u8]) -> Vec<usize> {
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
        sha2::Digest::update(&mut hasher, [i as u8]);
        let hash = hasher.finalize();
        let rand_byte = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let j = (rand_byte % (i as u64 + 1)) as usize;
        indices.swap(i, j);
        state = hash.to_vec();
    }

    indices[..k].to_vec()
}

pub fn get_randomness(session_id: SessionId) -> Vec<u8> {
    match session_id.prev() {
        Some(prev) => match DecKey::get(prev) {
            Ok(key) => key.into(),
            Err(_) => b"default-randomness".to_vec(),
        },
        None => {
            // Underflow means `initial session`
            return b"initial-randomness".to_vec();
        }
    }
}

pub fn verify_key_pair(
    skde_params: &SkdeParams,
    encryption_key: &str,
    decryption_key: &str,
) -> Result<(), KeyGenerationError> {
    let sample_message = "sample_message";

    let ciphertext = encrypt(skde_params, sample_message, encryption_key, true)
        .ok_or_trace()
        .ok_or_else(|| KeyGenerationError::InternalError("Encryption failed".into()))?;

    let decrypted_message = match decrypt(skde_params, &ciphertext, decryption_key) {
        Ok(message) => message,
        Err(err) => {
            tracing::error!("Decryption failed: {}", err);
            return Err(KeyGenerationError::InternalError(
                format!("Decryption failed: {}", err).into(),
            ));
        }
    };

    if decrypted_message.as_str() != sample_message {
        return Err(KeyGenerationError::InternalError(
            "Decryption failed: message mismatch".into(),
        ));
    }

    Ok(())
}
